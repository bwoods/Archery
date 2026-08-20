use crate::storage::{Entry, OccupiedEntry, VacantEntry};
use crate::{RecordBatch, StorageError};
use serde::{Serialize, de::DeserializeOwned};
use serde_arrow::schema::{SchemaLike, TracingOptions};

impl<T: Serialize + DeserializeOwned> Extend<T> for OccupiedEntry<'_> {
    /// # Panics
    /// Panics on any errors, as the `Extend` trait does not return a `Result`.
    ///
    /// See [`OccupiedEntry::try_extend`] for the panic-free version
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.try_extend(iter).unwrap();
    }
}

impl<'a> Entry<'a> {
    pub fn try_extend<T, I>(self, iter: I) -> Result<OccupiedEntry<'a>, StorageError>
    where
        I: IntoIterator<Item = T>,
        T: Serialize + DeserializeOwned,
    {
        match self {
            Entry::Occupied(mut entry) => {
                entry.try_extend(iter)?;
                Ok(entry)
            }
            Entry::Vacant(entry) => entry.try_extend(iter),
        }
    }
}

impl OccupiedEntry<'_> {
    pub fn try_extend<T, I>(&mut self, iter: I) -> Result<(), StorageError>
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

impl<'a> VacantEntry<'a> {
    pub fn try_extend<T, I>(self, iter: I) -> Result<OccupiedEntry<'a>, StorageError>
    where
        I: IntoIterator<Item = T>,
        T: Serialize + DeserializeOwned,
    {
        let mut occupied = self.into_occupied();
        occupied.try_extend(iter)?;
        Ok(occupied)
    }
}
