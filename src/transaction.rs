use super::entry::{Entry, OccupiedEntry, VacantEntry};
use redb::{CommitError, StorageError, TableError};
use redb::{ReadableTableMetadata, TableDefinition, WriteTransaction};

pub struct Txn {
    pub(crate) txn: WriteTransaction,
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
