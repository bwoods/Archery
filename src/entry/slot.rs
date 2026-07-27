use crate::Compression;
use crate::entry::occupied::Occupied;
use crate::error::Result;
use arrow_array::RecordBatch;
use jammdb::{Bucket, ToBytes};
use loom::compressors::FluxWriter;
use loom::{LoomCompressor, Predicate};
use std::iter::once;

pub(crate) struct Slot<'a, K> {
    pub(crate) compression: Compression,
    pub(crate) projection: Vec<String>,
    pub(crate) predicate: Predicate,
    pub(crate) parent: Bucket<'a, 'a>,
    pub(crate) key: K,
}

impl<'a, K> Slot<'a, K> {
    pub(crate) fn extend<I>(&self, iter: I) -> Result<()>
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
            return self.put(&self.parent, &self.key, batch); // single batches are stored directly under `key`
        }

        let bucket = self.parent.get_or_create_bucket(self.key.clone())?;
        self.append(&bucket, once(batch).chain(iter))
    }

    pub(crate) fn append<I>(&self, bucket: &Bucket<'a, 'a>, iter: I) -> Result<()>
    where
        I: IntoIterator<Item = RecordBatch>,
    {
        for batch in iter {
            let key = 1 + bucket.next_int(); // base₁
            self.put(&bucket, &key.to_be_bytes(), batch)?;
        }

        Ok(())
    }

    pub(crate) fn insert_entry(self, value: RecordBatch) -> Result<Occupied<'a, K>>
    where
        K: ToBytes<'a> + Clone,
    {
        self.put(&self.parent, &self.key, value)?;
        Ok(Occupied { slot: self })
    }

    pub(crate) fn put<Q>(&self, bucket: &Bucket<'a, 'a>, key: &Q, value: RecordBatch) -> Result<()>
    where
        Q: ToBytes<'a> + Clone, // nested keys may not by `K`
    {
        let writer = FluxWriter::with_profile(self.compression.into()).with_u64_only(true);
        bucket.put(key.clone(), writer.compress(&value)?)?;

        Ok(())
    }
}
