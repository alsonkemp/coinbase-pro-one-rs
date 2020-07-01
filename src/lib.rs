
extern crate chrono;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate hmac;
extern crate hyper;
extern crate hyper_tls;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate time;
extern crate tokio;
extern crate tokio_tungstenite;
extern crate uuid;
extern crate url;

pub mod client;
pub mod errors;
pub mod structs;
mod utils;

pub type Result<T> = std::result::Result<T, errors::CBError>;

pub const MAIN_URL: &str = "https://api.pro.coinbase.com";
pub const SANDBOX_URL: &str = "https://api-public.sandbox.pro.coinbase.com";
pub const WS_URL: &str = "wss://ws-feed.pro.coinbase.com";
pub const WS_SANDBOX_URL: &str = "wss://ws-feed-public.sandbox.pro.coinbase.com";
