use crate::Compression;
use crate::entry::occupied::get;
use crate::entry::{Entry, vacant::insert};
use crate::error::Result;
use arrow_array::RecordBatch;
use jammdb::{Bucket, Data, ToBytes};
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
            Entry::Vacant(vacant) => extend(
                &vacant.parent,
                vacant.key.clone(),
                vacant.compression,
                iter.into_iter(),
            ),
            Entry::Occupied(occupied) => match occupied.data()? {
                Data::Bucket(name) => {
                    let bucket = occupied.parent.get_bucket(name)?;
                    extend_many(&bucket, occupied.compression, iter)
                }
                Data::KeyValue(kv) => {
                    let current = get(kv.value())?;
                    occupied.parent.delete(occupied.name())?;

                    extend(
                        &occupied.parent,
                        occupied.key.clone(),
                        occupied.compression,
                        once(current).chain(iter.into_iter()),
                    )
                }
            },
        }
    }
}

pub(crate) fn extend<'a, K, I>(
    parent: &Bucket<'a, 'a>,
    key: K,
    compression: Compression,
    iter: I,
) -> Result<()>
where
    K: ToBytes<'a> + Clone,
    I: IntoIterator<Item = RecordBatch>,
{
    let mut iter = iter.into_iter().peekable();
    let batch = match iter.next() {
        Some(batch) => batch,
        None => return Ok(()),
    };

    if iter.peek().is_none() {
        return insert(&parent, &key, compression, batch); // single batches are stored directly under `key`
    }

    let bucket = extend_one(&parent, key, compression, batch)?;
    extend_many(&bucket, compression, iter)
}

pub(crate) fn extend_one<'a, K>(
    parent: &Bucket<'a, 'a>,
    key: K,
    compression: Compression,
    item: RecordBatch,
) -> Result<Bucket<'a, 'a>>
where
    K: ToBytes<'a> + Clone,
{
    let bucket = parent.get_or_create_bucket(key)?;
    extend_many(&bucket, compression, once(item))?;

    Ok(bucket)
}

pub(crate) fn extend_many<'a, I>(
    bucket: &Bucket<'a, 'a>,
    compression: Compression,
    iter: I,
) -> Result<()>
where
    I: IntoIterator<Item = RecordBatch>,
{
    for item in iter {
        let key = 1 + bucket.next_int(); // base₁
        insert(&bucket, &key.to_be_bytes(), compression, item)?;
    }

    Ok(())
}
