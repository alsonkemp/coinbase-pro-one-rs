use std::error::Error;
use std::fmt;

use serde_json;
use tokio_tungstenite::{tungstenite};

#[derive(Debug)]
pub enum CBProError {
    Http(String),
    Connect(tungstenite::Error),
    Send(tungstenite::Error),
    Read(tungstenite::Error),
    Serde(serde_json::Error, String ),
    Coinbase(CBError),
    Message(String),
    Null
}

impl fmt::Display for CBProError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for CBProError {}

#[derive(Debug)]
pub struct CBError {
    pub message: String,
}

impl Error for CBError {}

impl fmt::Display for CBError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

