use crate::entry::occupied::{empty_batch, get_only};
use crate::error::{Error, Result};
use arrow_array::RecordBatch;
use arrow_select::concat::concat_batches;
use fallible_iterator::FallibleIterator;
use jammdb::{Bucket, Cursor, Data, Error::IncompatibleValue, KVPair};
use loom::Predicate;
use serde::de::DeserializeOwned;

mod map_into;

pub use map_into::MapInto;

pub struct Iter<'a> {
    pub(crate) bucket: Bucket<'a, 'a>,
    pub(crate) predicate: Predicate,
    pub(crate) projection: Vec<String>,

    pub(crate) outer: Cursor<'a, 'a>,
    pub(crate) inner: Option<Cursor<'a, 'a>>,
}

impl<'a> FallibleIterator for Iter<'a> {
    type Item = RecordBatch;
    type Error = Error;

    fn next(&mut self) -> Result<Option<Self::Item>> {
        if let Some(kv) = self.next_kv()? {
            Ok(Some(get_only(
                kv.value(),
                &self.predicate,
                &self.projection,
            )?))
        } else {
            Ok(None)
        }
    }
}

impl<'a> Iter<'a> {
    /// - See [`itertools::Itertools::concat`] for comparison.
    /// - See [`arrow_select::concat::concat_batches`] for related warnings
    /// about memory usage and offset overflows.
    pub fn concat(self) -> Result<RecordBatch> {
        let batches: Vec<_> = self.collect()?;
        match batches.first().map(|first| first.schema()) {
            Some(schema) => Ok(concat_batches(&schema, &batches)?),
            None => Ok(empty_batch()),
        }
    }

    pub fn map_into<T: DeserializeOwned>(self) -> MapInto<'a, T> {
        MapInto {
            outer: self,
            inner: Default::default(),
        }
    }

    pub(crate) fn next_kv(&mut self) -> Result<Option<KVPair<'a, 'a>>> {
        match self.inner {
            Some(ref mut inner) => match inner.next() {
                Some(data) => match data {
                    Data::KeyValue(kv) => Ok(Some(kv)),
                    Data::Bucket(_) => Err(IncompatibleValue.into()),
                },
                None => {
                    self.inner = None;
                    self.next_kv() // recurse; grab the next outer
                }
            },
            None => match self.outer.next() {
                Some(data) => match data {
                    Data::KeyValue(kv) => Ok(Some(kv)),
                    Data::Bucket(name) => {
                        let bucket = self.bucket.get_bucket(name)?;
                        self.inner = Some(bucket.cursor());
                        self.next_kv() // recurse; grab the next inner
                    }
                },
                None => Ok(None),
            },
        }
    }
}
