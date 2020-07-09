use std::fmt;
use serde::{Deserialize, Deserializer};

#[derive(Serialize, Deserialize, Debug, Fail)]
pub enum Error {
    #[fail(display = "http: {}", _0)]
    Http(#[cause] super::hyper::Error),
    #[fail(display = "connect")]
    Connect(#[cause] super::tungstenite::Error),
    #[fail(display = "send") ]
    Send(#[cause] super::tungstenite::Error),
    #[fail(display = "read")]
    Read(#[cause] super::tungstenite::Error),
    #[fail(display = "serde: {}\n    {}", error, data)]
    Serde {
        #[cause]
        error: super::serde_json::Error,
        data: String,
    },
    #[fail(display = "coinbase: {}", _0)]
    Coinbase(Error),
    #[fail(display = "message: {}", _0)]
    Message(String),
    #[fail(display = "null")]
    Null,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl<'de> Deserialize<'de> for Error {
    fn deserialize<D>(_deserializer: D) -> Result<WSError, D::Error>
    where
        D: Deserializer<'de>,
    {
        unimplemented!()
    }
}
