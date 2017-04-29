use std;
use ignore;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Ignore(ignore::Error),
    StripPrefix(std::path::StripPrefixError),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<ignore::Error> for Error {
    fn from(err: ignore::Error) -> Error {
        Error::Ignore(err)
    }
}

impl From<std::path::StripPrefixError> for Error {
    fn from(err: std::path::StripPrefixError) -> Error {
        Error::StripPrefix(err)
    }
}
