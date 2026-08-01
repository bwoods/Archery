use super::entry::Entry;
use super::iter::Iter;
use crate::arrow::RecordBatch;
use fallible_iterator::FallibleIterator;
use loom::{Predicate, atlas::AtlasFooter};
use redb::{ReadableTableMetadata, StorageError};
use serde::{Deserialize, de::DeserializeOwned};
use serde_arrow::Deserializer;
use std::collections::VecDeque;
use std::ops::{Bound, RangeBounds};

impl<'a> Entry<'a> {
    #[inline(always)]
    pub fn range<T: DeserializeOwned>(
        &self,
        range: impl RangeBounds<usize>,
    ) -> Result<impl Iterator<Item = Result<T, StorageError>>, StorageError> {
        self.restrict(range, &[], Predicate::None)
    }

    pub fn restrict<T: DeserializeOwned>(
        &self,
        range: impl RangeBounds<usize>,
        projection: &[String],
        predicate: Predicate,
    ) -> Result<impl Iterator<Item = Result<T, StorageError>>, StorageError> {
        let table = match self {
            Entry::Occupied(entry) => &entry.table,
            Entry::Vacant(entry) => &entry.table,
        };

        let offset = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(min) => *min,
            Bound::Excluded(min) => min
                .checked_add(1)
                .ok_or(StorageError::ValueTooLarge(*min))?,
        };

        let length = match range.end_bound() {
            Bound::Unbounded => table.len()? as usize,
            Bound::Included(max) => *max,
            Bound::Excluded(max) => max
                .checked_add(1)
                .ok_or(StorageError::ValueTooLarge(*max))?,
        } - offset;

        let iter = Iter::new(table)?
            .projection(projection)
            .predicate(predicate);

        Ok(Range {
            iter,
            queue: Default::default(),
        }
        .skip(offset)
        .take(length))
    }
}

struct Range<'a, T> {
    queue: VecDeque<T>,
    iter: Iter<'a>,
}

impl<T> Iterator for Range<'_, T>
where
    T: DeserializeOwned,
{
    type Item = Result<T, StorageError>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.nth(0)
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        match Range::advance_by(self, n) {
            Err(err) => Some(Err(err)),
            Ok(0) => Ok(self.queue.pop_front()).transpose(),
            Ok(_) => None,
        }
    }
}

impl<T> Range<'_, T>
where
    T: DeserializeOwned,
{
    #[inline(never)]
    fn advance_by(&mut self, mut n: usize) -> Result<usize, StorageError> {
        if self.queue.is_empty() == false {
            if n < self.queue.len() {
                if n > 0 {
                    self.queue.drain(..n).for_each(drop);
                }

                return Ok(0);
            } else {
                n -= self.queue.len();
                self.queue.clear();
            }
        }

        let mut skipped: usize = 0;
        let found = self.iter.inner.find(|(_, bytes)| {
            let footer = AtlasFooter::from_file_tail(bytes.value())
                .map_err(|err| StorageError::Corrupted(err.to_string()))?;

            let count: usize = footer
                .blocks
                .iter()
                .map(|block| block.value_count as usize)
                .sum();

            if skipped + count > n {
                Ok(true)
            } else {
                skipped += count;
                Ok(false)
            }
        })?;

        let mut batch = match found {
            None => return Ok(n - skipped),
            Some((_, ref bytes)) => {
                RecordBatch::decompress(&self.iter.projection, &self.iter.predicate, bytes.value())
                    .map_err(|err| StorageError::Corrupted(err.to_string()))?
            }
        };

        if skipped != n {
            let start = n - skipped;
            let stop = batch.num_rows() - start;
            batch.0 = batch.slice(start, stop); // “Returns a zero-copy slice of this array…”
        }

        let deserializer = Deserializer::from_record_batch(&batch)
            .map_err(|err| StorageError::Corrupted(err.to_string()))?;

        self.queue.append(
            &mut VecDeque::<T>::deserialize(deserializer)
                .map_err(|err| StorageError::Corrupted(err.to_string()))?,
        );

        Ok(0)
    }
}
