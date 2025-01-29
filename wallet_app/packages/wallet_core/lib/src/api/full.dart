// This file is automatically generated, so please do not edit it.
// @generated by `flutter_rust_bridge`@ 2.7.0.

// ignore_for_file: invalid_use_of_internal_member, unused_import, unnecessary_import

import '../frb_generated.dart';
import '../models/attestation.dart';
import '../models/config.dart';
import '../models/disclosure.dart';
import '../models/instruction.dart';
import '../models/localize.dart';
import '../models/pin.dart';
import '../models/uri.dart';
import '../models/version_state.dart';
import '../models/wallet_event.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

// These functions are ignored because they are not marked as `pub`: `create_wallet`, `wallet`

Future<bool> isInitialized() => WalletCore.instance.api.crateApiFullIsInitialized();

Future<PinValidationResult> isValidPin({required String pin}) =>
    WalletCore.instance.api.crateApiFullIsValidPin(pin: pin);

Stream<bool> setLockStream() => WalletCore.instance.api.crateApiFullSetLockStream();

Future<void> clearLockStream() => WalletCore.instance.api.crateApiFullClearLockStream();

Stream<FlutterConfiguration> setConfigurationStream() => WalletCore.instance.api.crateApiFullSetConfigurationStream();

Future<void> clearConfigurationStream() => WalletCore.instance.api.crateApiFullClearConfigurationStream();

Stream<FlutterVersionState> setVersionStateStream() => WalletCore.instance.api.crateApiFullSetVersionStateStream();

Future<void> clearVersionStateStream() => WalletCore.instance.api.crateApiFullClearVersionStateStream();

Stream<List<Attestation>> setAttestationsStream() => WalletCore.instance.api.crateApiFullSetAttestationsStream();

Future<void> clearAttestationsStream() => WalletCore.instance.api.crateApiFullClearAttestationsStream();

Stream<List<WalletEvent>> setRecentHistoryStream() => WalletCore.instance.api.crateApiFullSetRecentHistoryStream();

Future<void> clearRecentHistoryStream() => WalletCore.instance.api.crateApiFullClearRecentHistoryStream();

Future<WalletInstructionResult> unlockWallet({required String pin}) =>
    WalletCore.instance.api.crateApiFullUnlockWallet(pin: pin);

Future<void> lockWallet() => WalletCore.instance.api.crateApiFullLockWallet();

Future<WalletInstructionResult> checkPin({required String pin}) =>
    WalletCore.instance.api.crateApiFullCheckPin(pin: pin);

Future<WalletInstructionResult> changePin({required String oldPin, required String newPin}) =>
    WalletCore.instance.api.crateApiFullChangePin(oldPin: oldPin, newPin: newPin);

Future<WalletInstructionResult> continueChangePin({required String pin}) =>
    WalletCore.instance.api.crateApiFullContinueChangePin(pin: pin);

Future<bool> hasRegistration() => WalletCore.instance.api.crateApiFullHasRegistration();

Future<void> register({required String pin}) => WalletCore.instance.api.crateApiFullRegister(pin: pin);

Future<IdentifyUriResult> identifyUri({required String uri}) =>
    WalletCore.instance.api.crateApiFullIdentifyUri(uri: uri);

Future<String> createPidIssuanceRedirectUri() => WalletCore.instance.api.crateApiFullCreatePidIssuanceRedirectUri();

Future<void> cancelPidIssuance() => WalletCore.instance.api.crateApiFullCancelPidIssuance();

Future<List<Attestation>> continuePidIssuance({required String uri}) =>
    WalletCore.instance.api.crateApiFullContinuePidIssuance(uri: uri);

Future<WalletInstructionResult> acceptPidIssuance({required String pin}) =>
    WalletCore.instance.api.crateApiFullAcceptPidIssuance(pin: pin);

Future<bool> hasActivePidIssuanceSession() => WalletCore.instance.api.crateApiFullHasActivePidIssuanceSession();

Future<StartDisclosureResult> startDisclosure({required String uri, required bool isQrCode}) =>
    WalletCore.instance.api.crateApiFullStartDisclosure(uri: uri, isQrCode: isQrCode);

Future<String?> cancelDisclosure() => WalletCore.instance.api.crateApiFullCancelDisclosure();

Future<AcceptDisclosureResult> acceptDisclosure({required String pin}) =>
    WalletCore.instance.api.crateApiFullAcceptDisclosure(pin: pin);

Future<bool> hasActiveDisclosureSession() => WalletCore.instance.api.crateApiFullHasActiveDisclosureSession();

Future<bool> isBiometricUnlockEnabled() => WalletCore.instance.api.crateApiFullIsBiometricUnlockEnabled();

Future<void> setBiometricUnlock({required bool enable}) =>
    WalletCore.instance.api.crateApiFullSetBiometricUnlock(enable: enable);

Future<void> unlockWalletWithBiometrics() => WalletCore.instance.api.crateApiFullUnlockWalletWithBiometrics();

Future<List<WalletEvent>> getHistory() => WalletCore.instance.api.crateApiFullGetHistory();

Future<List<WalletEvent>> getHistoryForCard({required String docType}) =>
    WalletCore.instance.api.crateApiFullGetHistoryForCard(docType: docType);

Future<void> resetWallet() => WalletCore.instance.api.crateApiFullResetWallet();

Future<String> getVersionString() => WalletCore.instance.api.crateApiFullGetVersionString();
