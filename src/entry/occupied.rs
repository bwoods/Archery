use crate::entry::slot::Slot;
use crate::error::{Error, Result};
use arrow_array::RecordBatch;
use jammdb::{Data, ToBytes};

pub struct Occupied<'a, K> {
    pub(crate) slot: Slot<'a, K>,
}

#[doc(hidden)]
impl<'a, K> std::ops::Deref for Occupied<'a, K> {
    type Target = Slot<'a, K>;

    fn deref(&self) -> &Self::Target {
        &self.slot
    }
}

/// See [`std::collections::btree_map::OccupiedEntry`] for comparison.
impl<'a, K> Occupied<'a, K>
where
    K: ToBytes<'a> + Clone,
{
    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn insert_entry(self, value: RecordBatch) -> Result<Occupied<'a, K>> {
        self.slot.insert_entry(value)
    }

    pub fn remove(self) -> Result<()> {
        self.parent.delete(self.key.clone().to_bytes())?;
        Ok(())
    }

    /// See the notes on  [`concat_batches`] for related warnings about
    /// memory usages and offset overflows.
    pub fn get(&self) -> Result<RecordBatch> {
        match self.data()? {
            Data::KeyValue(kv) => self.slot.get(kv),
            Data::Bucket(name) => {
                let bucket = self.parent.get_bucket(name)?;
                self.slot.concat(&bucket)
            }
        }
    }

    /// A copy of `key` that may be passed into functions expecting an
    /// `AsRef<[u8]>` (such as [`Bucket::get`]).
    pub(crate) fn name(&self) -> impl AsRef<[u8]> {
        self.key.clone().to_bytes()
    }

    /// The [`Bucket`] or [`KVPair`] at this entry.
    pub(crate) fn data(&self) -> Result<Data<'a, 'a>> {
        let data = self
            .parent
            .get(self.name())
            .ok_or_else(|| Error::Storage("An `Occupied` entry was empty?".to_string()))?;

        Ok(data)
    }
}
