use crate::entry::occupied::Occupied;
use crate::entry::slot::Slot;
use crate::error::Result;
use arrow_array::RecordBatch;
use jammdb::ToBytes;

/// See [`std::collections::btree_map::VacantEntry`] for comparison.
pub struct Vacant<'a, K> {
    pub(crate) slot: Slot<'a, K>,
}

impl<'a, K> Vacant<'a, K> {
    pub fn key(&self) -> &K {
        &self.slot.key
    }

    pub fn insert_entry(self, value: RecordBatch) -> Result<Occupied<'a, K>>
    where
        K: ToBytes<'a> + Clone,
    {
        self.slot.insert_entry(value)
    }
}
