use std;
use ignore;
#[cfg(feature = "cbor")]
use serde_cbor;
#[cfg(feature = "json")]
use serde_json;
#[cfg(feature = "msgpack")]
use rmp_serde;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    StripPrefix(std::path::StripPrefixError),
    Ignore(ignore::Error),
    #[cfg(feature = "cbor")]
    Cbor(serde_cbor::Error),
    #[cfg(feature = "json")]
    Json(serde_json::Error),
    #[cfg(feature = "msgpack")]
    MsgPackEncode(rmp_serde::encode::Error),
    #[cfg(feature = "msgpack")]
    MsgPackDecode(rmp_serde::decode::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<std::path::StripPrefixError> for Error {
    fn from(err: std::path::StripPrefixError) -> Error {
        Error::StripPrefix(err)
    }
}

impl From<ignore::Error> for Error {
    fn from(err: ignore::Error) -> Error {
        Error::Ignore(err)
    }
}

#[cfg(feature = "cbor")]
impl From<serde_cbor::Error> for Error {
    fn from(err: serde_cbor::Error) -> Error {
        Error::Cbor(err)
    }
}

#[cfg(feature = "json")]
impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::Json(err)
    }
}

#[cfg(feature = "msgpack")]
impl From<rmp_serde::encode::Error> for Error {
    fn from(err: rmp_serde::encode::Error) -> Error {
        Error::MsgPackEncode(err)
    }
}

#[cfg(feature = "msgpack")]
impl From<rmp_serde::decode::Error> for Error {
    fn from(err: rmp_serde::decode::Error) -> Error {
        Error::MsgPackDecode(err)
    }
}
