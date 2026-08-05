#[derive(Debug)]
#[non_exhaustive]
pub enum StorageError {
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

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        StorageError::Io(error)
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

impl From<nut::Error> for StorageError {
    fn from(error: nut::Error) -> Self {
        match error {
            nut::Error::IncompatibleValue
            | nut::Error::AllocationFailed
            | nut::Error::BucketNotFound
            | nut::Error::BucketExists
            | nut::Error::NameRequired
            | nut::Error::KeyRequired
            | nut::Error::KeyTooLarge
            | nut::Error::ValueTooLarge
            | nut::Error::ReadInProgress
            | nut::Error::WriteInProgress => StorageError::MisUse(error.to_string()),

            nut::Error::CheckFail(vec) => StorageError::CheckFail(vec),
            _ => StorageError::Corrupted(error.to_string()),
        }
    }
}
