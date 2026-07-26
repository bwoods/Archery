use crate::error::Error;
use arrow_array::RecordBatch;
use arrow_select::coalesce::BatchCoalescer;
use fallible_iterator::{FallibleIterator, from_fn};
use jammdb::Bucket;
use loom::Predicate;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_arrow::Deserializer;

pub mod flatten;
pub mod generator;

pub(crate) fn iter<'a, T: DeserializeOwned>(
    parent: &Bucket<'a, 'a>,
    key: impl AsRef<[u8]>,
    predicate: &Predicate,
    projection: &[String],
) -> impl FallibleIterator<Item = T, Error = Error> {
    generator::flat_map(parent, key, predicate, projection, |batch| {
        let deserializer = Deserializer::from_record_batch(&batch)?;
        Ok(Vec::<T>::deserialize(deserializer)?)
    })
}

/// See [`BatchCoalescer::with_biggest_coalesce_batch_size`] for discussion of
/// how the `limit` parameter may be used.
pub(crate) fn chunk<'a, T: DeserializeOwned>(
    parent: &Bucket<'a, 'a>,
    key: impl AsRef<[u8]>,
    predicate: &Predicate,
    projection: &[String],
    size: usize,
    limit: Option<usize>,
) -> impl FallibleIterator<Item = RecordBatch, Error = Error> {
    let mut iter = generator::flatten(parent, key, predicate, projection);
    let mut coalescer: Option<BatchCoalescer> = None;

    from_fn(move || {
        loop {
            match (iter.next()?, coalescer.as_mut()) {
                (Some(next), Some(buffered)) => {
                    buffered.push_batch(next)?; // next batch; do we have more than `size`?
                    if buffered.has_completed_batch() {
                        return Ok(buffered.next_completed_batch());
                    }
                }
                (Some(first), None) => {
                    // first batch; use its schema to create the BatchCoalescer
                    let coalescer = coalescer.insert(
                        BatchCoalescer::new(first.schema(), size)
                            .with_biggest_coalesce_batch_size(limit),
                    );
                    coalescer.push_batch(first)?;
                }
                (None, Some(buffered)) => {
                    buffered.finish_buffered_batch()?; // last batch; finish up
                    let last = buffered.next_completed_batch();
                    coalescer = None; // ⬇︎ will be hit on any subsequent calls
                    return Ok(last);
                }
                (None, None) => return Ok(None),
            }
        }
    })
}
