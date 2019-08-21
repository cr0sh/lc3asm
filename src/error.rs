//! Provides [Error] type for error handling.
use super::Rule;
use pest::error::Error as PestError;
use std::fmt::Error as FmtError;
use std::io::Error as IOError;
use std::num::ParseIntError;
use std::str::Utf8Error;

/// Assembler-related error type.
#[derive(Debug)]
pub enum Error {
    Pest(PestError<Rule>),
    ParseInt(ParseIntError),
    Io(IOError),
    Utf8(Utf8Error),
    Fmt(FmtError),
}

impl From<PestError<Rule>> for Error {
    fn from(e: PestError<Rule>) -> Error {
        Error::Pest(e)
    }
}

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Error {
        Error::ParseInt(e)
    }
}

impl From<IOError> for Error {
    fn from(e: IOError) -> Error {
        Error::Io(e)
    }
}

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Error {
        Error::Utf8(e)
    }
}

impl From<FmtError> for Error {
    fn from(e: FmtError) -> Error {
        Error::Fmt(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            // FIXME: why intellij-rust tries to match fmt with Debug when err.fmt(f)?
            Error::Pest(err) => std::fmt::Display::fmt(err, f),
            Error::ParseInt(err) => err.fmt(f),
            Error::Io(err) => err.fmt(f),
            Error::Utf8(err) => err.fmt(f),
            Error::Fmt(err) => err.fmt(f),
        }
    }
}
