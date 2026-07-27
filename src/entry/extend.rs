use crate::entry::Entry;
use crate::entry::occupied::get;
use crate::error::Result;
use arrow_array::RecordBatch;
use jammdb::{Data, ToBytes};
use std::iter::once;

impl<'tx, K> Entry<'tx, K>
where
    K: ToBytes<'tx> + Clone,
{
    /// Not an actual implementation of [`Extend`] as the trait does not return
    /// a `Result`.
    pub fn extend<T>(&mut self, iter: T) -> Result<()>
    where
        T: IntoIterator<Item = RecordBatch>,
    {
        match self {
            Entry::Vacant(vacant) => vacant.extend(iter.into_iter()),
            Entry::Occupied(occupied) => match occupied.data()? {
                Data::Bucket(name) => {
                    let bucket = occupied.parent.get_bucket(name)?;
                    occupied.extend_many(&bucket, iter)
                }
                Data::KeyValue(kv) => {
                    let current = get(kv.value())?;
                    occupied.parent.delete(occupied.name())?;
                    occupied.extend(once(current).chain(iter.into_iter()))
                }
            },
        }
    }
}
