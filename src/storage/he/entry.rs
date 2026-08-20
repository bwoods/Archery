use super::file::Txn;
use crate::structs::arrow::RecordBatch;
use crate::traits::deref::DerefCell;
use crate::{Compression, StorageError};
use heed::RwTxn;
use heed::byteorder::BigEndian;
use heed::types::{Bytes, U32};

pub(crate) type Table = heed::Database<U32<BigEndian>, Bytes>;

impl Txn<'_> {
    pub fn entry(&mut self, name: &str) -> Result<Entry<'_>, StorageError> {
        let key = name.to_owned();
        let table = self.env.create_database(&mut self.txn, Some(name))?;

        let entry = match table.len(&mut self.txn)? > 0 {
            false => Entry::Vacant(VacantEntry {
                table,
                txn: self.env.nested_write_txn(&mut self.txn)?.into(),
                key,
            }),
            true => Entry::Occupied(OccupiedEntry {
                table,
                txn: self.env.nested_write_txn(&mut self.txn)?.into(),
                key,
            }),
        };

        Ok(entry)
    }
}

pub enum Entry<'a> {
    Occupied(OccupiedEntry<'a>),
    Vacant(VacantEntry<'a>),
}

impl<'a> Entry<'a> {
    pub fn or_insert(self, default: RecordBatch) -> Result<OccupiedEntry<'a>, StorageError> {
        match self {
            Entry::Occupied(entry) => Ok(entry),
            Entry::Vacant(entry) => entry.insert_entry(default),
        }
    }

    pub fn or_insert_with<F>(self, default: F) -> Result<OccupiedEntry<'a>, StorageError>
    where
        F: FnOnce() -> RecordBatch,
    {
        match self {
            Entry::Occupied(entry) => Ok(entry),
            Entry::Vacant(entry) => entry.insert_entry(default()),
        }
    }

    pub fn or_insert_with_key<F>(self, default: F) -> Result<OccupiedEntry<'a>, StorageError>
    where
        F: FnOnce(&str) -> RecordBatch,
    {
        match self {
            Entry::Occupied(entry) => Ok(entry),
            Entry::Vacant(entry) => {
                let key = entry.key().to_owned();
                entry.insert_entry(default(&key))
            }
        }
    }

    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> &str {
        match self {
            Entry::Occupied(entry) => entry.key(),
            Entry::Vacant(entry) => entry.key(),
        }
    }

    pub fn insert_entry(self, value: RecordBatch) -> Result<OccupiedEntry<'a>, StorageError> {
        let entry = match self {
            Entry::Vacant(entry) => entry.insert_entry(value)?,
            Entry::Occupied(mut entry) => {
                entry.insert_entry(value)?;
                entry
            }
        };

        Ok(entry)
    }
}

pub struct OccupiedEntry<'a> {
    pub(crate) table: Table,
    pub(crate) txn: DerefCell<RwTxn<'a>>,
    pub(crate) key: String,
}

impl Drop for OccupiedEntry<'_> {
    fn drop(&mut self) {
        if let Some(txn) = self.txn.get() {
            txn.commit().expect("OccupiedEntry::drop::commit");
        }
    }
}

impl<'a> OccupiedEntry<'a> {
    pub fn insert_entry(
        &mut self,
        value: RecordBatch,
    ) -> Result<&mut OccupiedEntry<'a>, StorageError> {
        let key = self
            .table
            .last(&self.txn)?
            .map(|(k, _)| k)
            .unwrap_or_default()
            + 1;

        self.insert(key, value)
    }

    /// `remove` does not actually destroy the table entry; it just empties it.
    ///
    /// `vacuum` will be needed to completely  remove the empty table.
    pub fn remove_entry(mut self) -> Result<VacantEntry<'a>, StorageError> {
        self.table.clear(&mut self.txn)?;
        Ok(VacantEntry {
            table: self.table,
            txn: self.txn.take().into(),
            key: std::mem::take(&mut self.key),
        })
    }

    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> &str {
        &self.key
    }

    pub(crate) fn insert(
        &mut self,
        key: u32,
        value: RecordBatch,
    ) -> Result<&mut OccupiedEntry<'a>, StorageError> {
        let bytes = RecordBatch::compress(Compression::Good, &value)
            .map_err(|err| StorageError::Corrupted(err.to_string()))?;

        self.table.put(&mut self.txn, &key, bytes.as_ref())?;
        Ok(self)
    }
}

pub struct VacantEntry<'a> {
    pub(crate) table: Table,
    pub(crate) txn: DerefCell<RwTxn<'a>>,
    pub(crate) key: String,
}

impl Drop for VacantEntry<'_> {
    fn drop(&mut self) {
        if let Some(txn) = self.txn.get() {
            txn.commit().expect("VacantEntry::drop::commit");
        }
    }
}

impl<'a> VacantEntry<'a> {
    /// Sets the value of the entry with the `VacantEntry`’s key, and returns an `OccupiedEntry`.
    pub fn insert_entry(mut self, value: RecordBatch) -> Result<OccupiedEntry<'a>, StorageError> {
        let mut occupied = self.into_occupied();
        occupied.insert_entry(value)?;
        Ok(occupied)
    }

    /// Take ownership of the key.
    pub fn into_key(self) -> String {
        self.key.clone()
    }

    /// Gets a reference to the key that would be used when inserting a value through the VacantEntry.
    pub fn key(&self) -> &str {
        &self.key
    }

    pub(crate) fn into_occupied(mut self) -> OccupiedEntry<'a> {
        OccupiedEntry {
            table: self.table,
            txn: self.txn.take().into(),
            key: std::mem::take(&mut self.key),
        }
    }
}
