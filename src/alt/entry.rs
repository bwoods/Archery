use crate::Compression;
use crate::arrow::RecordBatch;
use jammdb::{DB, Error};
use std::marker::PhantomData;
use std::sync::Arc;

pub(crate) type Table<'a> = jammdb::Bucket<'a, 'a>;

pub enum Entry<'a> {
    Occupied(OccupiedEntry<'a>),
    Vacant(VacantEntry<'a>),
}

impl<'a> Entry<'a> {
    pub fn or_insert(self, default: RecordBatch) -> Result<OccupiedEntry<'a>, Error> {
        match self {
            Entry::Occupied(entry) => Ok(entry),
            Entry::Vacant(entry) => entry.insert_entry(default),
        }
    }

    pub fn or_insert_with<F>(self, default: F) -> Result<OccupiedEntry<'a>, Error>
    where
        F: FnOnce() -> RecordBatch,
    {
        match self {
            Entry::Occupied(entry) => Ok(entry),
            Entry::Vacant(entry) => entry.insert_entry(default()),
        }
    }

    pub fn or_insert_with_key<F>(self, default: F) -> Result<OccupiedEntry<'a>, Error>
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

    pub fn insert_entry(self, value: RecordBatch) -> Result<OccupiedEntry<'a>, Error> {
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
    pub(crate) db: Arc<DB>,
    pub(crate) key: String,
    pub(crate) _marker: PhantomData<&'a ()>,
}

impl<'a> OccupiedEntry<'a> {
    pub fn insert_entry(&mut self, value: RecordBatch) -> Result<&mut OccupiedEntry<'a>, Error> {
        self.insert(None, value)?;
        Ok(self)
    }

    /// `remove` does not actually destroy the table entry; it just empties it.
    ///
    /// `vacuum` will be needed to completely  remove the empty table.
    pub fn remove_entry(mut self) -> Result<VacantEntry<'a>, Error> {
        let tx = self.db.tx(true)?;
        let table = tx.get_bucket(self.key())?;

        for kv in table.kv_pairs() {
            table.delete(kv.key())?;
        }

        tx.commit()?;

        Ok(VacantEntry {
            db: self.db,
            key: self.key,
            _marker: Default::default(),
        })
    }

    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> &str {
        &self.key
    }

    pub(crate) fn insert(
        &mut self,
        key: Option<u32>,
        value: RecordBatch,
    ) -> Result<&mut OccupiedEntry<'a>, Error> {
        let bytes = RecordBatch::compress(Compression::Good, &value)
            .map_err(|err| Error::InvalidDB(err.to_string()))?;
        let value = Vec::from(bytes.as_ref());

        let tx = self.db.tx(true)?;
        let table = tx.get_bucket(self.key())?;
        let key = key
            .unwrap_or_else(|| table.next_int() as u32 + 1)
            .to_be_bytes();

        table.put(key, value)?;
        tx.commit()?;

        Ok(self)
    }
}

pub struct VacantEntry<'a> {
    pub(crate) db: Arc<DB>,
    pub(crate) key: String,
    pub(crate) _marker: PhantomData<&'a ()>,
}

impl<'a> VacantEntry<'a> {
    /// Sets the value of the entry with the `VacantEntry`’s key, and returns an `OccupiedEntry`.
    pub fn insert_entry(self, value: RecordBatch) -> Result<OccupiedEntry<'a>, Error> {
        let mut occupied = OccupiedEntry {
            db: self.db,
            key: self.key,
            _marker: Default::default(),
        };

        occupied.insert_entry(value)?;
        Ok(occupied)
    }

    /// Take ownership of the key.
    pub fn into_key(self) -> String {
        self.key().to_owned()
    }

    /// Gets a reference to the key that would be used when inserting a value through the VacantEntry.
    pub fn key(&self) -> &str {
        &self.key
    }
}
