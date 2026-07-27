use crate::error::Result;
use arrow_array::RecordBatch;
use jammdb::ToBytes;
use occupied::Occupied;
use vacant::Vacant;

pub use occupied::Occupied as OccupiedEntry;
pub use vacant::Vacant as VacantEntry;

mod extend;
mod occupied;
mod slot;
mod vacant;

/// A view into a entry, which may either be vacant or occupied.
pub enum Entry<'tx, K> {
    /// A vacant entry.
    Vacant(Vacant<'tx, K>),
    /// An occupied entry.
    Occupied(Occupied<'tx, K>),
}

impl<'tx, K: ToBytes<'tx> + Clone> Entry<'tx, K> {
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(occupied) => occupied.key(),
            Entry::Vacant(vacant) => vacant.key(),
        }
    }

    /// Ensures a value is in the entry by inserting the default if empty, and
    /// returns an `Occupied` entry.
    #[inline(always)]
    pub fn or_insert(self, default: RecordBatch) -> Result<Occupied<'tx, K>> {
        self.or_insert_with(|| default)
    }

    /// Ensures a value is in the entry by inserting the result of the default
    /// function if empty, and returns an `Occupied` entry.
    #[inline(always)]
    pub fn or_insert_with<F>(self, default: F) -> Result<Occupied<'tx, K>>
    where
        F: FnOnce() -> RecordBatch,
    {
        self.or_insert_with_key(|_| default())
    }

    /// Ensures a value is in the entry by inserting, if empty, the result of the default function.
    ///
    /// This method allows for generating key-derived values for insertion by providing the default
    /// function a reference to the key that was moved during the .entry(key) method call.
    #[inline]
    pub fn or_insert_with_key<F>(self, default: F) -> Result<Occupied<'tx, K>>
    where
        F: FnOnce(&K) -> RecordBatch,
    {
        match self {
            Self::Occupied(occupied) => Ok(occupied),
            Self::Vacant(vacant) => {
                let value = default(&vacant.slot.key);
                vacant.insert_entry(value)
            }
        }
    }

    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the map.
    ///
    /// See the notes on  [`arrow_select::concat::concat_batches`] for the
    /// warnings about memory safety and offset overflows.
    pub fn and_modify<F>(self, f: F) -> Result<Self>
    where
        F: FnOnce(&mut RecordBatch),
    {
        match self {
            Self::Occupied(occupied) => {
                let mut value = occupied.get()?;
                f(&mut value);
                Ok(Entry::Occupied(occupied.insert_entry(value)?))
            }
            vacant => Ok(vacant),
        }
    }

    /// Sets the value of the entry, and returns an `Occupied` entry.
    #[inline]
    pub fn insert(self, value: RecordBatch) -> Result<Occupied<'tx, K>> {
        match self {
            Self::Occupied(occupied) => occupied.insert_entry(value),
            Self::Vacant(vacant) => vacant.insert_entry(value),
        }
    }
}
