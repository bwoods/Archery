use crate::Compression;
use crate::entry::occupied::Occupied;
use crate::error::Result;
use arrow_array::RecordBatch;
use arrow_schema::Schema;
use arrow_select::concat::concat_batches;
use arrow_select::filter::filter_record_batch;
use fallible_iterator::FallibleIterator;
use fallible_iterator::IteratorExt;
use jammdb::{Bucket, KVPair, ToBytes, ToKVPairs};
use loom::compressors::FluxWriter;
use loom::decompressors::FluxReader;
use loom::{LoomCompressor, LoomDecompressor, Predicate};
use std::iter::once;

/// Commonality between `Vacant` and `Occupied` entries.
#[doc(hidden)]
pub struct Slot<'a, K> {
    pub(crate) compression: Compression,
    pub(crate) projection: Vec<String>,
    pub(crate) predicate: Predicate,
    pub(crate) parent: Bucket<'a, 'a>,
    pub(crate) key: K,
}

impl<'a, K> Slot<'a, K> {
    pub fn key(&self) -> &K {
        &self.key
    }

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

    pub(crate) fn get(&self, kv: KVPair) -> Result<RecordBatch> {
        let reader = FluxReader::new("");
        let batch = if self.projection.is_empty() {
            reader.decompress(kv.value(), &self.predicate)
        } else {
            reader.decompress_projected(kv.value(), &self.predicate, &self.projection)
        }
        .and_then(|batch| {
            if matches!(self.predicate, Predicate::None) {
                Ok(batch)
            } else {
                let mask = self.predicate.eval_on_batch(&batch)?;
                filter_record_batch(&batch, &mask).map_err(|err| err.into())
            }
        })?;

        Ok(batch)
    }

    pub(crate) fn concat(&self, bucket: &Bucket) -> Result<RecordBatch> {
        let batches: Vec<_> = bucket
            .cursor()
            .to_kv_pairs()
            .map(|kv| self.get(kv))
            .transpose_into_fallible()
            .collect()?;

        match batches.first().map(|first| first.schema()) {
            Some(schema) => Ok(concat_batches(&schema, &batches)?),
            None => Ok(self.empty_batch()),
        }
    }

    pub(crate) fn empty_batch(&self) -> RecordBatch {
        RecordBatch::new_empty(std::sync::Arc::new(Schema::empty()))
    }
}
