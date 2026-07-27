use crate::Compression;
use crate::entry::vacant::insert_entry;
use crate::error::{Error, Result};
use arrow_array::RecordBatch;
use arrow_schema::Schema;
use arrow_select::{concat::concat_batches, filter::filter_record_batch};
use fallible_iterator::{FallibleIterator, IteratorExt};
use jammdb::{Bucket, Data, ToBytes, ToKVPairs};
use loom::{LoomDecompressor, Predicate, decompressors::FluxReader};

pub struct Occupied<'a, K> {
    pub(crate) compression: Compression,
    pub(crate) parent: Bucket<'a, 'a>,
    pub(crate) key: K,
}

impl<'a, K> Occupied<'a, K>
where
    K: ToBytes<'a> + Clone,
{
    pub fn with_profile(self, profile: Compression) -> Self {
        Self {
            compression: profile,
            ..self
        }
    }

    pub fn key(&self) -> K {
        self.key.clone()
    }

    pub fn insert(self, value: RecordBatch) -> Result<Occupied<'a, K>> {
        insert_entry(self.parent, self.key, self.compression, value)
    }

    pub fn remove(self) -> Result<()> {
        self.parent.delete(self.key.to_bytes())?;
        Ok(())
    }

    pub fn get_all(&self) -> Result<RecordBatch> {
        self.get(&Predicate::None, &[])
    }

    /// See the notes on  [`concat_batches`] for related warnings about
    /// memory usages and offset overflows.
    pub fn get(&self, predicate: &Predicate, projection: &[String]) -> Result<RecordBatch> {
        match self.data()? {
            Data::KeyValue(kv) => get_only(kv.value(), predicate, projection),
            Data::Bucket(name) => {
                let bucket = self.parent.get_bucket(name)?;
                concat(&bucket, predicate, projection)
            }
        }
    }

    /// A copy of `key` that may be passed into functions expecting an
    /// `AsRef<[u8]>` (such as [`Bucket::get`]).
    pub(crate) fn name(&self) -> impl AsRef<[u8]> {
        self.key().to_bytes()
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

pub(crate) fn get(from: &[u8]) -> Result<RecordBatch> {
    get_only(from, &Predicate::None, &[])
}

pub(crate) fn get_only(
    from: &[u8],
    predicate: &Predicate,
    projection: &[String],
) -> Result<RecordBatch> {
    let reader = FluxReader::new("");
    let batch = if projection.is_empty() {
        reader.decompress(from, &predicate)
    } else {
        reader.decompress_projected(from, &predicate, projection)
    }
    .and_then(|batch| {
        if matches!(predicate, Predicate::None) {
            Ok(batch)
        } else {
            let mask = predicate.eval_on_batch(&batch)?;
            filter_record_batch(&batch, &mask).map_err(|err| err.into())
        }
    })?;

    Ok(batch)
}

pub(crate) fn concat<'a>(
    bucket: &Bucket<'a, 'a>,
    predicate: &Predicate,
    projection: &[String],
) -> Result<RecordBatch> {
    let batches: Vec<_> = bucket
        .cursor()
        .to_kv_pairs()
        .map(|kv| get_only(kv.value(), predicate, projection))
        .transpose_into_fallible()
        .collect()?;

    match batches.first().map(|first| first.schema()) {
        Some(schema) => Ok(concat_batches(&schema, &batches)?),
        None => Ok(empty_batch()),
    }
}

pub(crate) fn empty_batch() -> RecordBatch {
    RecordBatch::new_empty(std::sync::Arc::new(Schema::empty()))
}
