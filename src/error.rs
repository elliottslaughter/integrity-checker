use std;
use ignore;
use serde_json;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    StripPrefix(std::path::StripPrefixError),
    Ignore(ignore::Error),
    Json(serde_json::Error),
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

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::Json(err)
    }
}
