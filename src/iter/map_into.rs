use crate::error::{Error, Result};
use crate::iter::Iter;
use fallible_iterator::FallibleIterator;
use loom::atlas::AtlasFooter;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_arrow::Deserializer;
use std::collections::VecDeque;

pub struct MapInto<'a, K, T> {
    pub(crate) outer: Iter<'a, K>,
    pub(crate) inner: VecDeque<T>,
}

impl<'a, K, T> FallibleIterator for MapInto<'a, K, T>
where
    T: DeserializeOwned,
{
    type Item = T;
    type Error = Error;

    fn next(&mut self) -> Result<Option<Self::Item>> {
        self.nth(0)
    }

    fn nth(&mut self, mut n: usize) -> Result<Option<Self::Item>> {
        if self.inner.is_empty() == false {
            if n < self.inner.len() {
                if n > 0 {
                    self.inner.drain(..n).for_each(drop);
                }
                return Ok(self.inner.pop_front());
            } else {
                n -= self.inner.len();
                self.inner.clear();
            }
        }

        let mut skipped: usize = 0;
        let kv = loop {
            match self.outer.next_kv()? {
                None => return Ok(None),
                Some(kv) => {
                    let footer = AtlasFooter::from_file_tail(kv.value())?;
                    let count: usize = footer
                        .blocks
                        .iter()
                        .map(|block| block.value_count as usize)
                        .sum();

                    if skipped + count > n {
                        break kv;
                    } else {
                        skipped += count;
                    }
                }
            }
        };

        let mut batch = self.outer.slot.get(kv)?;
        if skipped != n {
            let start = n - skipped;
            let stop = batch.num_rows() - start;
            batch = batch.slice(start, stop); // “Returns a zero-copy slice of this array…”
        }

        let deserializer = Deserializer::from_record_batch(&batch)?;
        self.inner
            .append(&mut VecDeque::<T>::deserialize(deserializer)?);

        Ok(self.inner.pop_front())
    }
}
