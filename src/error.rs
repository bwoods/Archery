use arrow_schema::ArrowError;
use loom::FluxError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Arrow(ArrowError),
    Compression(String),
    Serialization(String),
    Storage(String),
    Io(std::io::Error),
}

impl From<ArrowError> for Error {
    fn from(error: ArrowError) -> Self {
        match error {
            ArrowError::IoError(_, error) => Self::Io(error),
            error => Self::Arrow(error),
        }
    }
}

impl From<FluxError> for Error {
    fn from(error: FluxError) -> Self {
        match error {
            FluxError::Arrow(string) => Self::Arrow(string),
            FluxError::Io(error) => Self::Io(error),
            error => Self::Compression(error.to_string()),
        }
    }
}

impl From<jammdb::Error> for Error {
    fn from(error: jammdb::Error) -> Self {
        match error {
            jammdb::Error::Io(error) => Self::Io(error),
            error => Self::Storage(error.to_string()),
        }
    }
}

impl From<serde_arrow::Error> for Error {
    fn from(error: serde_arrow::Error) -> Self {
        Self::Serialization(error.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
