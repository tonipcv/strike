pub mod http_client;
pub mod api_fuzzer;
pub mod browser;
pub mod websocket;
pub mod secret_scanner;
pub mod traffic_replayer;
pub mod dns_resolver;
pub mod port_scanner;

pub use http_client::*;
pub use api_fuzzer::*;
pub use websocket::*;
pub use secret_scanner::*;
pub use dns_resolver::*;
pub use port_scanner::*;

// Re-export only specific items to avoid ambiguous names between modules
pub use browser::{BrowserDriver, Cookie, PageSnapshot};
pub use traffic_replayer::{ReplayResult, ResponseDiff, RecordedRequest, RecordedResponse, TrafficRecord, TrafficReplayer, MutationStrategy};
