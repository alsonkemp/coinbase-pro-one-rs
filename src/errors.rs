use std::error::Error;
use std::fmt;

#[derive(Debug, PartialOrd, PartialEq)]
pub enum CBProError {
    Coinbase(CBError),
    Connect(String),
    Http(String),
    Message(String),
    Null,
    Read(String),
    Send(String),
    Serde(String ),
}

impl fmt::Display for CBProError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for CBProError {}

#[derive(Debug, PartialOrd, PartialEq)]
pub struct CBError(String);

impl Error for CBError {}

impl fmt::Display for CBError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

