//! `PlatformPublisher` implementations (section 7). Real providers live
//! under `infrastructure::connectors::{youtube,tiktok,kwai}` alongside
//! their existing auth/connector code; `fake_publisher` is the
//! first-class scripted provider every engine test and future dev
//! simulation mode uses instead (section 142/161).

pub mod fake_publisher;
pub mod http_client;
pub mod stub_publisher;

pub use fake_publisher::{FakePublisher, FakeScenario};
pub use stub_publisher::StubPublisher;
