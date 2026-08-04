use crate::entry::{Entry, OccupiedEntry, VacantEntry};
use redb::{Builder, Database, DatabaseStats, Error, StorageError, backends::InMemoryBackend};
use redb::{CommitError, DatabaseError, TableError, TransactionError};
use redb::{ReadableTableMetadata, TableDefinition, TableHandle, WriteTransaction};
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

    pub fn compact(&mut self) -> Result<bool, Error> {
        let txn = self.db.begin_write()?;

        for table in txn.list_tables()? {
            let name = TableDefinition::<'_, u32, &'static [u8]>::new(table.name());
            let table = txn.open_table(name)?;
            if table.is_empty()? {
                txn.delete_table(name)?;
            }
        }

        txn.commit()?;

        self.db.compact().map_err(|err| err.into())
    }

    pub fn stats(&self) -> Result<DatabaseStats, TransactionError> {
        let txn = self.db.begin_write()?;
        Ok(txn.stats()?)
    }
}

pub struct Txn {
    txn: WriteTransaction,
}

impl Txn {
    pub fn commit(self) -> Result<(), CommitError> {
        self.txn.commit()
    }

    pub fn rollback(self) -> Result<(), StorageError> {
        self.txn.abort()
    }

    pub fn entry(&self, name: &str) -> Result<Entry<'_>, TableError> {
        let definition = TableDefinition::new(name);
        let table = self.txn.open_table(definition)?;

        let entry = match table.is_empty()? {
            true => Entry::Vacant(VacantEntry { table }),
            false => Entry::Occupied(OccupiedEntry { table }),
        };

        Ok(entry)
    }
}
