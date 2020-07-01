use serde::{Serialize, Deserialize};
// use std::fmt;

#[derive(Debug, Deserialize, Fail, Serialize)]
pub enum Error {
    #[fail(display = "connect")]
    Connect (#[cause] super::tokio_tungstenite::tungstenite::Error),
    #[fail(display = "message")]
    Message (String),
    #[fail(display = "read")]
    Read (#[cause] super::tokio_tungstenite::tungstenite::Error),
    #[fail(display = "send")]
    Send (#[cause] super::tokio_tungstenite::tungstenite::Error),
    #[fail(display = "serde")]
    Serde {
        #[cause]
        error: super::serde_json::Error,
        data: String
    }
}

//impl<'de> Deserialize<'de> for Error {
//    fn deserialize<D>(_deserializer: D) -> Result<Error, D::Error>
//        where
//            D: Deserializer<'de>,
//    {
//        unimplemented!()
//    }
//}

//impl fmt::Display for self::Error {
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        write!(f, "{}", self.message)
//    }
//}

#[derive(Debug, Fail)]
pub enum CBError {
    #[fail(display = "http: {}", _0)]
    Http(#[cause] super::hyper::Error),
    #[fail(display = "serde: {}\n    {}", error, data)]
    Serde {
        #[cause]
        error: super::serde_json::Error,
        data: String,
    },
    #[fail(display = "coinbase: {}", _0)]
    Coinbase(Error),
    #[fail(display = "null")]
    Null,
}
