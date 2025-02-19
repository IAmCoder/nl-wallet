use std::collections::HashSet;

use chrono::DateTime;
use chrono::TimeDelta;
use chrono::Utc;
use derive_more::AsRef;

use super::integrity_verdict::AppLicensingVerdict;
use super::integrity_verdict::AppRecognitionVerdict;
use super::integrity_verdict::DeviceRecognitionVerdict;
use super::integrity_verdict::IntegrityVerdict;

// The oldest an integrity verdict request can be is 15 minutes.
const MAX_REQUEST_AGE: TimeDelta = TimeDelta::minutes(15);

// To prevent clocks skew issues to some degree, have some margin
// when determining that a request timestamp is in the future.
const FUTURE_SKEW_MARGIN: TimeDelta = TimeDelta::minutes(5);

#[derive(Debug, thiserror::Error)]
pub enum IntegrityVerdictVerificationError {
    #[error("request package name does not match, received: {0}")]
    RequestPackageNameMismatch(String),
    #[error("request hash does not match")]
    RequestHashMismatch,
    #[error("integrity verdict was requested too long ago or in the future: {0}")]
    RequestTimestampInvalid(DateTime<Utc>),
    #[error("the app and certificate do not match the version distributed by Google Play: {0}")]
    NotPlayRecognized(AppRecognitionVerdict),
    #[error("integrity verdict package name does not match, received: {}", .0.as_deref().unwrap_or("<NONE>"))]
    PlayStorePackageNameMismatch(Option<String>),
    #[error("the set of play store certificate hashes in the integrity verdict do not match those provided")]
    PlayStoreCertificateMismatch,
    #[error("the user did not install the app from Google Play, received: {0}")]
    NoAppEntitlement(AppLicensingVerdict),
    #[error("the device does not pass system integrity checks or does not meet Android compatibility requirements")]
    DeviceIntegrityNotMet,
}

#[derive(Debug, Clone)]
pub enum VerifyPlayStore {
    NoVerify,
    Verify {
        play_store_certificate_hashes: HashSet<Vec<u8>>,
    },
}

#[derive(Debug, Clone, AsRef)]
pub struct VerifiedIntegrityVerdict(IntegrityVerdict);

/// Wraps a verified instance of [`IntegrityVerdict`]. The verification is done according to recommendations in the
/// [Google documentation](https://developer.android.com/google/play/integrity/verdicts). It does not consider opt-in
/// fields. If the `verify_play_store` parameter is `VerifyPlayStore::Verify`, extra values will be checked and
/// verification will only succeed if the app was downloaded from the Play Store. This should not be used in a local
/// development environment.
impl VerifiedIntegrityVerdict {
    pub fn verify(
        integrity_verdict: IntegrityVerdict,
        package_name: &str,
        request_hash: &[u8],
        verify_play_store: &VerifyPlayStore,
    ) -> Result<Self, IntegrityVerdictVerificationError> {
        Self::verify_with_time(
            integrity_verdict,
            package_name,
            request_hash,
            verify_play_store,
            Utc::now(),
        )
    }

    pub fn verify_with_time(
        integrity_verdict: IntegrityVerdict,
        package_name: &str,
        request_hash: &[u8],
        verify_play_store: &VerifyPlayStore,
        time: DateTime<Utc>,
    ) -> Result<Self, IntegrityVerdictVerificationError> {
        if integrity_verdict.request_details.request_package_name != package_name {
            return Err(IntegrityVerdictVerificationError::RequestPackageNameMismatch(
                integrity_verdict.request_details.request_package_name,
            ));
        }

        if integrity_verdict.request_details.request_hash != request_hash {
            return Err(IntegrityVerdictVerificationError::RequestHashMismatch);
        }

        // This is sensitive to clock skews on the host machine. As this will also reject timestamps
        // that are in the future, we apply some amount of margin here in that direction.
        let request_time_delta = time.signed_duration_since(integrity_verdict.request_details.timestamp);
        if request_time_delta > MAX_REQUEST_AGE || request_time_delta < -FUTURE_SKEW_MARGIN {
            return Err(IntegrityVerdictVerificationError::RequestTimestampInvalid(
                integrity_verdict.request_details.timestamp,
            ));
        }

        if let VerifyPlayStore::Verify {
            play_store_certificate_hashes,
        } = verify_play_store
        {
            if integrity_verdict.app_integrity.app_recognition_verdict != AppRecognitionVerdict::PlayRecognized {
                return Err(IntegrityVerdictVerificationError::NotPlayRecognized(
                    integrity_verdict.app_integrity.app_recognition_verdict,
                ));
            }

            if integrity_verdict
                .app_integrity
                .details
                .as_ref()
                .map(|details| details.package_name.as_str())
                != Some(package_name)
            {
                return Err(IntegrityVerdictVerificationError::PlayStorePackageNameMismatch(
                    integrity_verdict
                        .app_integrity
                        .details
                        .map(|details| details.package_name),
                ));
            }

            if integrity_verdict
                .app_integrity
                .details
                .as_ref()
                .map(|details| &details.certificate_sha256_digest)
                != Some(play_store_certificate_hashes)
            {
                return Err(IntegrityVerdictVerificationError::PlayStoreCertificateMismatch);
            }

            if integrity_verdict.account_details.app_licensing_verdict != AppLicensingVerdict::Licensed {
                return Err(IntegrityVerdictVerificationError::NoAppEntitlement(
                    integrity_verdict.account_details.app_licensing_verdict,
                ));
            }
        }

        if !integrity_verdict
            .device_integrity
            .device_recognition_verdict
            .contains(&DeviceRecognitionVerdict::MeetsDeviceIntegrity)
        {
            return Err(IntegrityVerdictVerificationError::DeviceIntegrityNotMet);
        }

        Ok(Self(integrity_verdict))
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use chrono::NaiveDate;
    use rstest::rstest;

    use super::super::tests::EXAMPLE_VERDICT;
    use super::*;

    fn verify_example_verdict(
        integrity_verdict: IntegrityVerdict,
        verify_play_store: bool,
    ) -> Result<(), IntegrityVerdictVerificationError> {
        VerifiedIntegrityVerdict::verify_with_time(
            integrity_verdict,
            "com.package.name",
            b"hello wolrd there",
            &match verify_play_store {
                false => VerifyPlayStore::NoVerify,
                true => VerifyPlayStore::Verify {
                    play_store_certificate_hashes: HashSet::from([
                        b"\x6a\x6a\x14\x74\xb5\xcb\xbb\x2b\x1a\xa5\x7e\x0b\xc3".to_vec(),
                    ]),
                },
            },
            NaiveDate::from_ymd_opt(2023, 2, 6)
                .unwrap()
                .and_hms_opt(3, 45, 0)
                .unwrap()
                .and_utc(),
        )
        .map(|_| ())
    }

    #[rstest]
    fn test_verified_integrity_verdict(#[values(true, false)] verify_play_store: bool) {
        verify_example_verdict(EXAMPLE_VERDICT.clone(), verify_play_store)
            .expect("integrity verdict should verify successfully");
    }

    #[rstest]
    fn test_verified_integrity_verdict_request_package_name_mismatch_error(
        #[values(true, false)] verify_play_store: bool,
    ) {
        let mut verdict = EXAMPLE_VERDICT.clone();
        verdict.request_details.request_package_name = "com.package.different".to_string();

        let error =
            verify_example_verdict(verdict, verify_play_store).expect_err("integrity verdict should not verify");

        assert_matches!(
            error,
            IntegrityVerdictVerificationError::RequestPackageNameMismatch(name) if name == "com.package.different"
        )
    }

    #[rstest]
    fn test_verified_integrity_verdict_request_hash_mismatch_error(#[values(true, false)] verify_play_store: bool) {
        let mut verdict = EXAMPLE_VERDICT.clone();
        verdict.request_details.request_hash = b"different_hash".to_vec();

        let error =
            verify_example_verdict(verdict, verify_play_store).expect_err("integrity verdict should not verify");

        assert_matches!(error, IntegrityVerdictVerificationError::RequestHashMismatch)
    }

    #[rstest]
    // Too long ago.
    #[case(NaiveDate::from_ymd_opt(2023, 2, 6).unwrap().and_hms_opt(3, 25, 0).unwrap().and_utc(), false)]
    // Within the max timestamp age.
    #[case(NaiveDate::from_ymd_opt(2023, 2, 6).unwrap().and_hms_opt(3, 35, 0).unwrap().and_utc(), true)]
    // In the future, within the acceptable margin.
    #[case(NaiveDate::from_ymd_opt(2023, 2, 6).unwrap().and_hms_opt(3, 47, 0).unwrap().and_utc(), true)]
    // Too far into the future.
    #[case(NaiveDate::from_ymd_opt(2023, 2, 6).unwrap().and_hms_opt(3, 51, 0).unwrap().and_utc(), false)]
    fn test_verified_integrity_verdict_request_timestamp_inconsistent_error(
        #[values(true, false)] verify_play_store: bool,
        #[case] verdict_timestamp: DateTime<Utc>,
        #[case] should_succeed: bool,
    ) {
        let mut verdict = EXAMPLE_VERDICT.clone();
        verdict.request_details.timestamp = verdict_timestamp;

        // Note that the timestamp is checked against a "current" time of 2024-02-06T03:45:00Z.
        let result = verify_example_verdict(verdict, verify_play_store);

        if should_succeed {
            result.expect("integrity verdict should verify successfully");
        } else {
            assert_matches!(
                result,
                Err(IntegrityVerdictVerificationError::RequestTimestampInvalid(date)) if date == verdict_timestamp
            )
        }
    }

    #[test]
    fn test_verified_integrity_verdict_not_play_recognized_error() {
        let mut verdict = EXAMPLE_VERDICT.clone();
        verdict.app_integrity.app_recognition_verdict = AppRecognitionVerdict::UnrecognizedVersion;

        verify_example_verdict(verdict.clone(), false).expect("integrity verdict should verify successfully");

        let error = verify_example_verdict(verdict, true).expect_err("integrity verdict should not verify");

        assert_matches!(
            error,
            IntegrityVerdictVerificationError::NotPlayRecognized(recognition_verdict)
                if recognition_verdict == AppRecognitionVerdict::UnrecognizedVersion
        )
    }

    #[test]
    fn test_verified_integrity_verdict_play_store_package_name_mismatch_error() {
        let mut verdict = EXAMPLE_VERDICT.clone();
        verdict.app_integrity.details.as_mut().unwrap().package_name = "com.package.different".to_string();

        verify_example_verdict(verdict.clone(), false).expect("integrity verdict should verify successfully");

        let error = verify_example_verdict(verdict, true).expect_err("integrity verdict should not verify");

        assert_matches!(
            error,
            IntegrityVerdictVerificationError::PlayStorePackageNameMismatch(name)
                if name == Some("com.package.different".to_string())
        )
    }

    #[test]
    fn test_verified_integrity_verdict_play_store_certificate_mismatch_error() {
        let mut verdict = EXAMPLE_VERDICT.clone();
        verdict
            .app_integrity
            .details
            .as_mut()
            .unwrap()
            .certificate_sha256_digest
            .clear();

        verify_example_verdict(verdict.clone(), false).expect("integrity verdict should verify successfully");

        let error = verify_example_verdict(verdict, true).expect_err("integrity verdict should not verify");

        assert_matches!(error, IntegrityVerdictVerificationError::PlayStoreCertificateMismatch)
    }

    #[test]
    fn test_verified_integrity_verdict_no_app_entitlement_error() {
        let mut verdict = EXAMPLE_VERDICT.clone();
        verdict.account_details.app_licensing_verdict = AppLicensingVerdict::Unlicensed;

        verify_example_verdict(verdict.clone(), false).expect("integrity verdict should verify successfully");

        let error = verify_example_verdict(verdict, true).expect_err("integrity verdict should not verify");

        assert_matches!(
            error,
            IntegrityVerdictVerificationError::NoAppEntitlement(license_verdict)
                if license_verdict == AppLicensingVerdict::Unlicensed
        )
    }

    #[rstest]
    fn test_verified_integrity_verdict_device_integrity_not_met_error(#[values(true, false)] verify_play_store: bool) {
        let mut verdict = EXAMPLE_VERDICT.clone();
        verdict.device_integrity.device_recognition_verdict.clear();

        let error =
            verify_example_verdict(verdict, verify_play_store).expect_err("integrity verdict should not verify");

        assert_matches!(error, IntegrityVerdictVerificationError::DeviceIntegrityNotMet)
    }
}
