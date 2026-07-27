use crate::entry::Entry;
use crate::error::Result;
use arrow_array::RecordBatch;
use jammdb::{Data, ToBytes};
use std::iter::once;

impl<'a, K> Entry<'a, K>
where
    K: ToBytes<'a> + Clone,
{
    /// Not an actual implementation of [`Extend`] as the trait does not return
    /// a `Result`.
    pub fn extend<I>(&mut self, iter: I) -> Result<()>
    where
        I: IntoIterator<Item = RecordBatch>,
    {
        match self {
            Entry::Vacant(vacant) => vacant.slot.extend(iter.into_iter()),
            Entry::Occupied(occupied) => match occupied.data()? {
                Data::Bucket(name) => {
                    let bucket = occupied.slot.parent.get_bucket(name)?;
                    occupied.slot.append(&bucket, iter)
                }
                Data::KeyValue(kv) => {
                    let current = occupied.get_kv(kv)?;
                    occupied
                        .slot
                        .parent
                        .delete(occupied.slot.key.clone().to_bytes())?;
                    occupied.slot.extend(once(current).chain(iter.into_iter()))
                }
            },
        }
    }
}
