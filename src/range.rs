use super::entry::Entry;
use super::iter::Iter;
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
        bounds: impl RangeBounds<usize>,
    ) -> Result<impl Iterator<Item = Result<T, StorageError>>, StorageError> {
        self.restrict(bounds, &[], Predicate::None)
    }

    pub fn restrict<T: DeserializeOwned>(
        &self,
        bounds: impl RangeBounds<usize>,
        projection: &[String],
        predicate: Predicate,
    ) -> Result<impl Iterator<Item = Result<T, StorageError>>, StorageError> {
        let iter = self.iter()?.projection(projection).predicate(predicate);

        let range = from_bounds(bounds);
        Ok(Range {
            iter,
            queue: Default::default(),
        }
        // using `dropping` here, rather than `skip`, would eagerly fill `queue`
        // on iterator creation and that feels… un-Rusty?
        .skip(range.start)
        .take(range.len()))
    }

    pub fn remove(&mut self, bounds: impl RangeBounds<usize>) -> Result<(), StorageError> {
        let range = from_bounds(bounds);
        let mut start = range.start;
        let mut count = range.len();

        let mut iter = self.iter()?;
        loop {
            start = Range::<()>::advance_inner(&mut iter, start)?;

            let mut batch = match iter.next() {
                None => return Ok(()),
                Some(Err(err)) => return Err(err),
                Some(Ok(batch)) => batch,
            };

            // batch.0 = match count.cmp(&(batch.num_rows() - start)) {
            //     Ordering::Less => 0,
            //     Ordering::Equal => {
            //         if start == 0 {
            //             batch.0
            //         } else {
            //             batch.slice(start, batch.num_rows() - start)
            //         }
            //     }
            //     Ordering::Greater => 0,
            // };
        }

        Ok(())
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

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        match self.advance_by(n) {
            Err(err) => Some(Err(err)),
            Ok(0) => Ok(self.queue.pop_front()).transpose(),
            Ok(_) => None,
        }
    }
}

impl<T> DoubleEndedIterator for Range<'_, T>
where
    T: DeserializeOwned,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl<T> Range<'_, T>
where
    T: DeserializeOwned,
{
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

        let k = Self::advance_inner(&mut self.iter, n)?;
        let mut batch = match self.iter.next().transpose()? {
            None => return Ok(k),
            Some(batch) => batch,
        };

        if k != 0 {
            batch.0 = batch.slice(k, batch.num_rows() - k); // “Returns a zero-copy slice of this array…”
        }

        let deserializer = Deserializer::from_record_batch(&batch)
            .map_err(|err| StorageError::Corrupted(err.to_string()))?;

        self.queue.append(
            &mut VecDeque::<T>::deserialize(deserializer)
                .map_err(|err| StorageError::Corrupted(err.to_string()))?,
        );

        Ok(0)
    }

    /// Advances the storage-level iterator until its `next()` will return the
    /// block that holds row `n` + 1.
    ///
    /// Returns the gap between the number of rows that were requested and the
    /// number that were actually dropped.
    #[inline(never)]
    fn advance_inner(iter: &mut Iter, n: usize) -> Result<usize, StorageError> {
        let mut skipped: usize = 0;

        let found = loop {
            match iter.inner.next() {
                Some(Ok(found)) => {
                    let footer = AtlasFooter::from_file_tail(found.1.value())
                        .map_err(|err| StorageError::Corrupted(err.to_string()))?;

                    let count: usize = footer
                        .blocks
                        .iter()
                        .map(|block| block.value_count as usize)
                        .sum();

                    if skipped + count > n {
                        break found;
                    } else {
                        skipped += count;
                        continue;
                    }
                }
                Some(Err(err)) => return Err(err),
                None => return Ok(n - skipped),
            }
        };

        iter.inner.put_back(Ok(found));
        Ok(0)
    }
}

fn from_bounds(bounds: impl RangeBounds<usize>) -> std::ops::Range<usize> {
    let start = match bounds.start_bound() {
        Bound::Unbounded => 0,
        Bound::Included(min) => *min,
        Bound::Excluded(min) => *min + 1,
    };

    let end = match bounds.end_bound() {
        Bound::Unbounded => usize::MAX,
        Bound::Included(max) => *max,
        Bound::Excluded(max) => *max + 1,
    };

    start..end
}
