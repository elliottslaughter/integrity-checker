use std;
use ignore;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Ignore(ignore::Error),
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
