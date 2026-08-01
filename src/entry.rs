use redb::{ReadableTable, StorageError, TableHandle};

pub(crate) type Table<'a> = redb::Table<'a, u32, &'static [u8]>;

pub enum Entry<'a> {
    Occupied(OccupiedEntry<'a>),
    Vacant(VacantEntry<'a>),
}

impl<'a> Entry<'a> {
    pub fn or_insert(self, default: &[u8]) -> Result<OccupiedEntry<'a>, StorageError> {
        match self {
            Entry::Occupied(entry) => Ok(entry),
            Entry::Vacant(entry) => entry.insert_entry(default),
        }
    }

    pub fn or_insert_with<F, R>(self, default: F) -> Result<OccupiedEntry<'a>, StorageError>
    where
        F: FnOnce() -> R,
        R: AsRef<[u8]>,
    {
        match self {
            Entry::Occupied(entry) => Ok(entry),
            Entry::Vacant(entry) => entry.insert_entry(default().as_ref()),
        }
    }

    pub fn or_insert_with_key<F, R>(self, default: F) -> Result<OccupiedEntry<'a>, StorageError>
    where
        F: FnOnce(&str) -> R,
        R: AsRef<[u8]>,
    {
        match self {
            Entry::Occupied(entry) => Ok(entry),
            Entry::Vacant(entry) => {
                let key = entry.key().to_owned();
                entry.insert_entry(default(&key).as_ref())
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

    pub fn insert_entry(self, value: &[u8]) -> Result<OccupiedEntry<'a>, StorageError> {
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
    pub(crate) table: Table<'a>,
}

impl<'a> OccupiedEntry<'a> {
    pub fn insert_entry(&mut self, value: &[u8]) -> Result<&mut OccupiedEntry<'a>, StorageError> {
        let key = self
            .table
            .last()?
            .map(|(k, _)| k.value())
            .unwrap_or_default()
            + 1;

        self.table.insert(key, value)?;
        Ok(self)
    }

    /// `remove` does not actually destroy the table entry; it just empties it.
    ///
    /// `vacuum` will be needed to completely  remove the empty table.
    pub fn remove_entry(mut self) -> Result<VacantEntry<'a>, StorageError> {
        self.table.retain(|_, _| false)?;
        Ok(VacantEntry { table: self.table })
    }

    pub fn key(&self) -> &str {
        self.table.name()
    }
}

pub struct VacantEntry<'a> {
    pub(crate) table: Table<'a>,
}

impl<'a> VacantEntry<'a> {
    /// Sets the value of the entry with the `VacantEntry`’s key, and returns an `OccupiedEntry`.
    pub fn insert_entry(self, value: &[u8]) -> Result<OccupiedEntry<'a>, StorageError> {
        let mut occupied = OccupiedEntry { table: self.table };
        occupied.insert_entry(value)?;
        Ok(occupied)
    }

    /// Take ownership of the key.
    pub fn into_key(self) -> String {
        self.table.name().to_owned()
    }

    /// Gets a reference to the key that would be used when inserting a value through the VacantEntry.
    pub fn key(&self) -> &str {
        self.table.name()
    }
}
