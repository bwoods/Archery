#[derive(Debug)]
#[non_exhaustive]
pub enum StorageError {
    /// Failures from various Arrow operations
    Arrow(arrow_schema::ArrowError),
    /// Error returned when the DB is found to be in an invalid state
    Corrupted(String),
    /// Wrapper around a [`std::io::Error`] that occurred while opening the file or writing to it
    Io(std::io::Error),

    CheckFail(Vec<String>),
    MisUse(String),
}

impl std::error::Error for StorageError {}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

impl From<arrow_schema::ArrowError> for StorageError {
    fn from(error: arrow_schema::ArrowError) -> Self {
        StorageError::Arrow(error)
    }
}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        StorageError::Io(error)
    }
}

impl From<tempfile::PersistError> for StorageError {
    fn from(error: tempfile::PersistError) -> Self {
        StorageError::Io(error.error)
    }
}

impl From<redb::StorageError> for StorageError {
    fn from(error: redb::StorageError) -> Self {
        match error {
            redb::StorageError::Corrupted(str) => StorageError::Corrupted(str),
            redb::StorageError::Io(io) => StorageError::Io(io),
            _ => StorageError::MisUse(error.to_string()),
        }
    }
}

impl From<redb::CommitError> for StorageError {
    fn from(error: redb::CommitError) -> Self {
        match error {
            redb::CommitError::Storage(err) => err.into(),
            _ => StorageError::MisUse(error.to_string()),
        }
    }
}

impl From<redb::CompactionError> for StorageError {
    fn from(error: redb::CompactionError) -> Self {
        match error {
            redb::CompactionError::Storage(err) => err.into(),
            _ => StorageError::MisUse(error.to_string()),
        }
    }
}

impl From<redb::DatabaseError> for StorageError {
    fn from(error: redb::DatabaseError) -> Self {
        match error {
            redb::DatabaseError::Storage(err) => err.into(),
            _ => StorageError::MisUse(error.to_string()),
        }
    }
}

impl From<redb::TableError> for StorageError {
    fn from(error: redb::TableError) -> Self {
        match error {
            redb::TableError::Storage(err) => err.into(),
            redb::TableError::TableAlreadyOpen(_, _) | redb::TableError::TableExists(_) => {
                StorageError::MisUse(error.to_string())
            }
            _ => StorageError::MisUse(error.to_string()),
        }
    }
}

impl From<redb::TransactionError> for StorageError {
    fn from(error: redb::TransactionError) -> Self {
        match error {
            redb::TransactionError::Storage(err) => err.into(),
            redb::TransactionError::ReadTransactionStillInUse(_) => {
                StorageError::MisUse(error.to_string())
            }
            _ => StorageError::MisUse(error.to_string()),
        }
    }
}

impl From<heed::Error> for StorageError {
    fn from(error: heed::Error) -> Self {
        match error {
            heed::Error::Io(io) => StorageError::Io(io),
            heed::Error::Mdb(_) | heed::Error::Encoding(_) | heed::Error::Decoding(_) => {
                StorageError::Corrupted(error.to_string())
            }
            heed::Error::EnvAlreadyOpened => StorageError::MisUse(error.to_string()),
        }
    }
}
