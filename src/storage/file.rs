use crate::StorageError;
use crate::storage::entry::{Entry, OccupiedEntry, VacantEntry};
use redb::{Builder, Database, DatabaseStats, backends::InMemoryBackend};
use redb::{ReadableTableMetadata, TableDefinition, TableHandle, WriteTransaction};
use std::path::Path;
use tempfile::NamedTempFile;

pub struct File {
    db: Database,
}

impl File {
    pub fn path(path: impl AsRef<Path>) -> Result<File, StorageError> {
        let db = Database::create(path)?;
        Ok(Self { db })
    }

    pub fn temporary() -> Result<File, StorageError> {
        let temp = NamedTempFile::new()?;
        Self::path(temp.path())
    }

    pub fn memory() -> Result<File, StorageError> {
        let db = Builder::new().create_with_backend(InMemoryBackend::new())?;
        Ok(Self { db })
    }

    pub fn txn(&self) -> Result<Txn, StorageError> {
        let txn = self.db.begin_write()?;
        Ok(Txn { txn })
    }

    pub fn compact(&mut self) -> Result<bool, StorageError> {
        let txn = self.db.begin_write()?;

        for table in txn.list_tables()? {
            let name = TableDefinition::<'_, u32, &'static [u8]>::new(table.name());
            let table = txn.open_table(name)?;
            if table.is_empty()? {
                txn.delete_table(name)?;
            }
        }

        txn.commit()?;

        Ok(self.db.compact()?)
    }

    pub fn stats(&self) -> Result<DatabaseStats, StorageError> {
        let txn = self.db.begin_write()?;
        Ok(txn.stats()?)
    }
}

pub struct Txn {
    txn: WriteTransaction,
}

impl Txn {
    pub fn commit(self) -> Result<(), StorageError> {
        self.txn.commit()?;
        Ok(())
    }

    pub fn rollback(self) -> Result<(), StorageError> {
        self.txn.abort()?;
        Ok(())
    }

    pub fn entry(&self, name: &str) -> Result<Entry<'_>, StorageError> {
        let definition = TableDefinition::new(name);
        let table = self.txn.open_table(definition)?;

        let entry = match table.is_empty()? {
            true => Entry::Vacant(VacantEntry { table }),
            false => Entry::Occupied(OccupiedEntry { table }),
        };

        Ok(entry)
    }
}
