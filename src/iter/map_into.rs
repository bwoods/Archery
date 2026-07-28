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
    pub(crate) queue: VecDeque<T>,
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
        if self.queue.is_empty() == false {
            if n < self.queue.len() {
                if n > 0 {
                    self.queue.drain(..n).for_each(drop);
                }
                return Ok(self.queue.pop_front());
            } else {
                n -= self.queue.len();
                self.queue.clear();
            }
        }

        let mut skipped: usize = 0;
        let kv = 'found: loop {
            match self.outer.next() {
                None => return Ok(None),
                Some(kv) => {
                    let footer = AtlasFooter::from_file_tail(kv.value())?;
                    let count: usize = footer
                        .blocks
                        .iter()
                        .map(|block| block.value_count as usize)
                        .sum();

                    if skipped + count > n {
                        break 'found kv;
                    } else {
                        skipped += count;
                    }
                }
            }
        };

        let mut batch = self.outer.occupied.get_kv(kv)?;
        if skipped != n {
            let start = n - skipped;
            let stop = batch.num_rows() - start;
            batch = batch.slice(start, stop); // “Returns a zero-copy slice of this array…”
        }

        let deserializer = Deserializer::from_record_batch(&batch)?;
        self.queue
            .append(&mut VecDeque::<T>::deserialize(deserializer)?);

        Ok(self.queue.pop_front())
    }
}
