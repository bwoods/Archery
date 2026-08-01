use crate::transaction::Txn;
use redb::{Builder, Database, DatabaseError, TransactionError, backends::InMemoryBackend};
use std::path::Path;
use tempfile::NamedTempFile;

pub struct File {
    db: Database,
}

impl File {
    pub fn path(path: impl AsRef<Path>) -> Result<File, DatabaseError> {
        let db = Database::create(path)?;
        Ok(Self { db })
    }

    pub fn temporary() -> Result<File, DatabaseError> {
        let temp = NamedTempFile::new()?;
        Self::path(temp.path())
    }

    pub fn memory() -> Result<File, DatabaseError> {
        let db = Builder::new().create_with_backend(InMemoryBackend::new())?;
        Ok(Self { db })
    }

    pub fn txn(&self) -> Result<Txn, TransactionError> {
        let txn = self.db.begin_write()?;
        Ok(Txn { txn })
    }
}
