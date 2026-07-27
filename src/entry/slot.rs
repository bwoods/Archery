use crate::entry::occupied::Occupied;
use crate::{Compression, error};
use arrow_array::RecordBatch;
use jammdb::{Bucket, ToBytes};
use loom::LoomCompressor;
use loom::compressors::FluxWriter;
use std::iter::once;

/// Commonality between `Vacant` and `Occupied` entries.
pub struct Slot<'a, K> {
    pub(crate) compression: Compression,
    pub(crate) parent: Bucket<'a, 'a>,
    pub(crate) key: K, // FIXME; move back out; Iter won’t need it…
}

impl<'a, K> Slot<'a, K>
where
    K: ToBytes<'a> + Clone,
{
    pub fn key(&self) -> K {
        self.key.clone()
    }

    pub(crate) fn insert_entry(self, value: RecordBatch) -> error::Result<Occupied<'a, K>> {
        self.insert(&self.parent, &self.key, value)?;
        Ok(Occupied { slot: self })
    }

    pub(crate) fn insert<Q>(
        &self,
        bucket: &Bucket<'a, 'a>,
        key: &Q,
        value: RecordBatch,
    ) -> error::Result<()>
    where
        Q: ToBytes<'a> + Clone, // nested keys may not by `K`
    {
        let writer = FluxWriter::with_profile(self.compression.into()).with_u64_only(true);
        bucket.put(key.clone(), writer.compress(&value)?)?;

        Ok(())
    }

    pub(crate) fn extend<I>(&self, iter: I) -> error::Result<()>
    where
        I: IntoIterator<Item = RecordBatch>,
    {
        let mut iter = iter.into_iter().peekable();
        let batch = match iter.next() {
            Some(batch) => batch,
            None => return Ok(()),
        };

        if iter.peek().is_none() {
            return self.insert(&self.parent, &self.key, batch); // single batches are stored directly under `key`
        }

        let bucket = self.extend_one(batch)?;
        self.extend_many(&bucket, iter)
    }

    pub(crate) fn extend_one(&self, item: RecordBatch) -> error::Result<Bucket<'a, 'a>> {
        let bucket = self.parent.get_or_create_bucket(self.key.clone())?;
        self.extend_many(&bucket, once(item))?;

        Ok(bucket)
    }

    pub(crate) fn extend_many<I>(&self, bucket: &Bucket<'a, 'a>, iter: I) -> error::Result<()>
    where
        I: IntoIterator<Item = RecordBatch>,
    {
        for batch in iter {
            let key = 1 + bucket.next_int(); // base₁
            self.insert(&bucket, &key.to_be_bytes(), batch)?;
        }

        Ok(())
    }
}
