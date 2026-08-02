use crate::arrow::RecordBatch;
use crate::entry::{Entry, OccupiedEntry, VacantEntry};
use redb::StorageError;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_arrow::schema::{SchemaLike, TracingOptions};

impl<T: Serialize + DeserializeOwned> Extend<T> for OccupiedEntry<'_> {
    /// # Panics
    /// Panics on any errors, as the `Extend` trait does not return a `Result`.
    ///
    /// See [`OccupiedEntry::append`] for the panic-free version
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.append(iter).unwrap();
    }
}

impl Entry<'_> {
    pub fn append<T, I>(self, iter: I) -> Result<(), StorageError>
    where
        I: IntoIterator<Item = T>,
        T: Serialize + DeserializeOwned,
    {
        match self {
            Entry::Occupied(mut entry) => entry.append(iter),
            Entry::Vacant(entry) => entry.append(iter),
        }
    }
}

impl OccupiedEntry<'_> {
    pub fn append<T, I>(&mut self, iter: I) -> Result<(), StorageError>
    where
        I: IntoIterator<Item = T>,
        T: Serialize + DeserializeOwned,
    {
        // “A common case is passing a vector into a function which immediately
        //  re-collects into a vector. We can short circuit this if the `IntoIter`
        //  has not been advanced at all.”
        let items = Vec::from_iter(iter);

        let batch = Vec::from_type::<T>(TracingOptions::default())
            .and_then(|fields| serde_arrow::to_record_batch(&fields, &items))
            .map_err(|err| StorageError::Corrupted(err.to_string()))
            .map(RecordBatch)?;

        self.insert_entry(batch)?;
        Ok(())
    }
}

impl VacantEntry<'_> {
    pub fn append<T, I>(self, iter: I) -> Result<(), StorageError>
    where
        I: IntoIterator<Item = T>,
        T: Serialize + DeserializeOwned,
    {
        let mut occupied = OccupiedEntry { table: self.table };
        occupied.append(iter)
    }
}
