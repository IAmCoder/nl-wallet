#[cfg(feature = "mock")]
pub mod mock {
    use chrono::{offset::TimeZone, DateTime, Utc};
    use uuid::{uuid, Uuid};
    use wallet_common::generator::Generator;

    pub struct FixedUuidGenerator;

    impl Generator<Uuid> for FixedUuidGenerator {
        fn generate(&self) -> Uuid {
            uuid!("c9723aef-022b-4ab7-9cc3-ff4227ec1cc9")
        }
    }

    pub struct EpochGenerator;

    impl Generator<DateTime<Utc>> for EpochGenerator {
        fn generate(&self) -> DateTime<Utc> {
            Utc.timestamp_nanos(0)
        }
    }

    pub struct MockGenerators;

    impl Generator<Uuid> for MockGenerators {
        fn generate(&self) -> Uuid {
            uuid!("c9723aef-022b-4ab7-9cc3-ff4227ec1cc9")
        }
    }

    impl Generator<DateTime<Utc>> for MockGenerators {
        fn generate(&self) -> DateTime<Utc> {
            Utc.timestamp_nanos(0)
        }
    }
}
