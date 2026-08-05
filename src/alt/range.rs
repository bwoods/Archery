use super::entry::Entry;
use super::iter::Iter;
use jammdb::{Data, Error};
use loom::{Predicate, atlas::AtlasFooter};
use serde::{Deserialize, de::DeserializeOwned};
use serde_arrow::Deserializer;
use std::collections::VecDeque;
use std::ops::{Bound, RangeBounds};

impl<'a> Entry<'a> {
    pub fn range<T: DeserializeOwned>(
        &'a self,
        bounds: impl RangeBounds<usize>,
    ) -> Result<impl Iterator<Item = Result<T, Error>>, Error> {
        self.restrict(bounds, &[], Predicate::None)
    }

    pub fn restrict<T: DeserializeOwned>(
        &'a self,
        bounds: impl RangeBounds<usize>,
        projection: &[String],
        predicate: Predicate,
    ) -> Result<impl Iterator<Item = Result<T, Error>>, Error> {
        let iter = self.iter()?.projection(projection).predicate(predicate);

        let range = from_bounds(bounds);
        Ok(Range {
            iter,
            queue: Default::default(),
        }
        // using `dropping` here, rather than `skip`, would eagerly fill `queue`
        // on iterator creation and that feels… un-Rust-like?
        .skip(range.start)
        .take(range.len()))
    }

    #[inline(never)]
    pub fn remove(&mut self, _bounds: impl RangeBounds<usize>) -> Result<(), Error> {
        unimplemented!()
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
    type Item = Result<T, Error>;

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

impl<T> Range<'_, T>
where
    T: DeserializeOwned,
{
    #[inline(never)]
    fn advance_by(&mut self, mut n: usize) -> Result<usize, Error> {
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

        let k = self.iter.advance_inner(n)?;
        let mut batch = match self.iter.next().transpose()? {
            None => return Ok(k),
            Some(batch) => batch,
        };

        if k != 0 {
            batch.0 = batch.slice(k, batch.num_rows() - k); // “Returns a zero-copy slice of this array…”
        }

        let deserializer = Deserializer::from_record_batch(&batch)
            .map_err(|err| Error::InvalidDB(err.to_string()))?;

        self.queue.append(
            &mut VecDeque::<T>::deserialize(deserializer)
                .map_err(|err| Error::InvalidDB(err.to_string()))?,
        );

        Ok(0)
    }
}

impl Iter<'_> {
    /// Advances the `Iter` until its `next()` will return the block that holds row `n` + 1.
    ///
    /// Returns the gap between `n` and the number of rows that were actually dropped.
    #[inline(never)]
    fn advance_inner(&mut self, n: usize) -> Result<usize, Error> {
        let mut skipped: usize = 0;

        loop {
            match self.inner.next() {
                Some(Data::KeyValue(kv)) => {
                    let footer = AtlasFooter::from_file_tail(kv.value())
                        .map_err(|err| Error::InvalidDB(err.to_string()))?;

                    let count: usize = footer
                        .blocks
                        .iter()
                        .map(|block| block.value_count as usize)
                        .sum();

                    if skipped + count > n {
                        self.inner.put_back(Data::KeyValue(kv));
                        return Ok(0);
                    } else {
                        skipped += count;
                        continue;
                    }
                }
                Some(Data::Bucket(_)) => return Err(Error::KeyValueMissing),
                None => return Ok(n - skipped),
            }
        }
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
