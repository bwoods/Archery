use crate::entry::occupied::Occupied;
use crate::entry::slot::Slot;
use crate::error::Result;
use arrow_array::RecordBatch;
use jammdb::ToBytes;

/// See [`std::collections::btree_map::VacantEntry`] for comparison.
pub struct Vacant<'a, K> {
    pub(crate) slot: Slot<'a, K>,
}

#[doc(hidden)]
impl<'a, K> std::ops::Deref for Vacant<'a, K> {
    type Target = Slot<'a, K>;

    fn deref(&self) -> &Self::Target {
        &self.slot
    }
}

impl<'a, K> Vacant<'a, K>
where
    K: ToBytes<'a> + Clone,
{
    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn insert_entry(self, value: RecordBatch) -> Result<Occupied<'a, K>> {
        self.slot.insert_entry(value)
    }
}
