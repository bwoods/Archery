use crate::entry::Slot;
use crate::error::{Error, Result};
use arrow_array::RecordBatch;
use fallible_iterator::FallibleIterator;
use jammdb::{Cursor, Data, Error::IncompatibleValue, KVPair, ToBytes};
pub use map_into::MapInto;
use serde::de::DeserializeOwned;

mod map_into;

pub struct Iter<'a, K> {
    pub(crate) outer: Cursor<'a, 'a>,
    pub(crate) inner: Option<Cursor<'a, 'a>>,

    pub(crate) slot: Slot<'a, K>,
}

impl<'a, K> FallibleIterator for Iter<'a, K> {
    type Item = RecordBatch;
    type Error = Error;

    fn next(&mut self) -> Result<Option<Self::Item>> {
        if let Some(kv) = self.next_kv()? {
            Ok(Some(self.slot.get(kv)?))
        } else {
            Ok(None)
        }
    }
}

impl<'a, K> Iter<'a, K> {
    /// - See [`itertools::concat`] for comparison.
    /// - See [`arrow_select::concat_batches`] for related warnings about memory usage and offset overflows.
    ///
    /// [`itertools::concat`]: https://docs.rs/itertools/latest/itertools/trait.Itertools.html#method.concat
    /// [`arrow_select::concat_batches`]: https://docs.rs/arrow-select/latest/arrow_select/concat/fn.concat_batches.html
    pub fn concat(self) -> Result<RecordBatch>
    where
        K: ToBytes<'a> + Clone,
    {
        self.slot.concat(&self.slot.parent)
    }

    pub fn map_into<T: DeserializeOwned>(self) -> MapInto<'a, K, T> {
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
                        let bucket = self.slot.parent.get_bucket(name)?;
                        self.inner = Some(bucket.cursor());
                        self.next_kv() // recurse; grab the next inner
                    }
                },
                None => Ok(None),
            },
        }
    }
}
