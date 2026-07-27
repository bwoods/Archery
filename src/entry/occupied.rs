use crate::entry::slot::Slot;
use crate::error::{Error, Result};
use arrow_array::RecordBatch;
use jammdb::{Data, ToBytes};

pub struct Occupied<'a, K> {
    pub(crate) slot: Slot<'a, K>,
}

/// See [`std::collections::btree_map::OccupiedEntry`] for comparison.
impl<'a, K> Occupied<'a, K> {
    pub fn key(&self) -> &K {
        &self.slot.key
    }

    pub fn insert_entry(self, value: RecordBatch) -> Result<Occupied<'a, K>>
    where
        K: ToBytes<'a> + Clone,
    {
        self.slot.insert_entry(value)
    }

    pub fn remove(self) -> Result<()>
    where
        K: ToBytes<'a> + Clone,
    {
        self.slot.parent.delete(self.slot.key.clone().to_bytes())?;
        Ok(())
    }

    /// - See [`concat_batches`] for related warnings about memory usage and offset overflows.
    ///
    /// [`concat_batches`]: https://docs.rs/arrow-select/latest/arrow_select/concat/fn.concat_batches.html
    pub fn get(&self) -> Result<RecordBatch>
    where
        K: ToBytes<'a> + Clone,
    {
        match self.data()? {
            Data::KeyValue(kv) => self.slot.get(kv),
            Data::Bucket(name) => {
                let bucket = self.slot.parent.get_bucket(name)?;
                self.slot.concat(&bucket)
            }
        }
    }

    /// The [`Bucket`] or [`KVPair`] at this entry.
    pub(crate) fn data(&self) -> Result<Data<'a, 'a>>
    where
        K: ToBytes<'a> + Clone,
    {
        let data = self
            .slot
            .parent
            .get(self.slot.key.clone().to_bytes())
            .ok_or_else(|| Error::Storage("An `Occupied` entry was empty?".to_string()))?;

        Ok(data)
    }
}
