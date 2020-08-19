
#![feature(box_syntax)]
#![feature(format_args_capture)]

extern crate async_std;
extern crate async_tls;
extern crate async_tungstenite;
// extern crate chrono;
extern crate crypto;
extern crate futures;
extern crate futures_util;
#[macro_use]
extern crate log;
extern crate ordered_float;
extern crate pretty_env_logger;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate time;
extern crate uuid;
extern crate url;

pub mod book;
pub mod conduit;
pub mod errors;
pub mod structs;
mod utils;

pub const MAIN_URL: &str = "https://api.pro.coinbase.com";
pub const SANDBOX_URL: &str = "https://api-public.sandbox.pro.coinbase.com";
pub const WS_URL: &str = "wss://ws-feed.pro.coinbase.com";
pub const WS_SANDBOX_URL: &str = "wss://ws-feed-public.sandbox.pro.coinbase.com";
