use crate::entry::slot::Slot;
use crate::error::{Error, Result};
use arrow_array::RecordBatch;
use arrow_schema::Schema;
use arrow_select::{concat::concat_batches, filter::filter_record_batch};
use fallible_iterator::{FallibleIterator, IteratorExt};
use jammdb::{Data, KVPair, ToBytes, ToKVPairs};
use loom::{LoomDecompressor, Predicate, decompressors::FluxReader};

pub struct Occupied<'a, K> {
    pub(crate) slot: Slot<'a, K>,
}

/// See [`std::collections::btree_map::OccupiedEntry`] for comparison.
impl<'a, K> Occupied<'a, K> {
    pub fn key(&self) -> &K {
        &self.slot.key
    }

    pub fn insert_entry(self, value: RecordBatch) -> Result<Occupied<'a, K>>
    where
        K: ToBytes<'a> + Clone,
    {
        self.slot.insert_entry(value)
    }

    pub fn remove(self) -> Result<()>
    where
        K: ToBytes<'a> + Clone,
    {
        self.slot.parent.delete(self.slot.key.clone().to_bytes())?;
        Ok(())
    }

    /// See [`concat_batches`] for related warnings about memory usage and offset overflows.
    ///
    /// [`concat_batches`]: https://docs.rs/arrow-select/latest/arrow_select/concat/fn.concat_batches.html
    pub fn get(&self) -> Result<RecordBatch>
    where
        K: ToBytes<'a> + Clone,
    {
        match self.data()? {
            Data::KeyValue(kv) => self.get_kv(kv),
            Data::Bucket(name) => {
                let bucket = self.slot.parent.get_bucket(name)?;

                let batches: Vec<_> = bucket
                    .cursor()
                    .to_kv_pairs()
                    .map(|kv| self.get_kv(kv))
                    .transpose_into_fallible()
                    .collect()?;

                match batches.first().map(|first| first.schema()) {
                    Some(schema) => Ok(concat_batches(&schema, &batches)?),
                    None => Ok(RecordBatch::new_empty(std::sync::Arc::new(Schema::empty()))),
                }
            }
        }
    }

    /// The [`Bucket`] or [`KVPair`] at this entry.
    pub(crate) fn data(&self) -> Result<Data<'a, 'a>>
    where
        K: ToBytes<'a> + Clone,
    {
        let data = self
            .slot
            .parent
            .get(self.slot.key.clone().to_bytes())
            .ok_or_else(|| Error::Storage("An `Occupied` entry was empty?".to_string()))?;

        Ok(data)
    }

    pub(crate) fn get_kv(&self, kv: KVPair) -> Result<RecordBatch> {
        let reader = FluxReader::new("");
        let batch = if self.slot.projection.is_empty() {
            reader.decompress(kv.value(), &self.slot.predicate)
        } else {
            reader.decompress_projected(kv.value(), &self.slot.predicate, &self.slot.projection)
        }
        .and_then(|batch| {
            if matches!(self.slot.predicate, Predicate::None) {
                Ok(batch)
            } else {
                let mask = self.slot.predicate.eval_on_batch(&batch)?;
                filter_record_batch(&batch, &mask).map_err(|err| err.into())
            }
        })?;

        Ok(batch)
    }
}
