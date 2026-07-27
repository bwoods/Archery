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
            Entry::Vacant(vacant) => vacant.extend(iter.into_iter()),
            Entry::Occupied(occupied) => match occupied.data()? {
                Data::Bucket(name) => {
                    let bucket = occupied.parent.get_bucket(name)?;
                    occupied.append(&bucket, iter)
                }
                Data::KeyValue(kv) => {
                    let current = occupied.slot.get(kv)?;
                    occupied.parent.delete(occupied.name())?;
                    occupied.extend(once(current).chain(iter.into_iter()))
                }
            },
        }
    }
}
