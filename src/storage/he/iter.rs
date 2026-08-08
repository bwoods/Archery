use super::entry::{Entry, Table};
use crate::{RecordBatch, StorageError};
use heed::byteorder::BigEndian;
use heed::types::{Bytes, U32};
use heed::{RoIter, RwTxn};
use itertools::{PutBack, put_back};
use loom::Predicate;
use ouroboros::self_referencing;

impl<'a> Entry<'a> {
    pub fn into_iter(self) -> Result<Iter<'a>, StorageError> {
        match self {
            Entry::Occupied(mut entry) => Iter::from(entry.table, entry.txn.take().unwrap()),
            Entry::Vacant(mut entry) => Iter::from(entry.table, entry.txn.take().unwrap()),
        }
    }
}

#[self_referencing]
pub struct Iter<'a> {
    pub(crate) txn: RwTxn<'a>,
    #[borrows(mut txn)]
    #[not_covariant]
    pub(crate) inner: PutBack<RoIter<'this, U32<BigEndian>, Bytes>>,

    pub(crate) projection: Vec<String>,
    pub(crate) predicate: Predicate,
}

impl<'a> Iter<'a> {
    pub(crate) fn from(table: Table, txn: RwTxn<'a>) -> Result<Self, StorageError> {
        IterTryBuilder {
            txn,
            inner_builder: |txn| Ok(put_back(table.iter(txn)?)),
            projection: Vec::default(),
            predicate: Predicate::None,
        }
        .try_build()
    }

    pub fn projection(mut self, projection: &[String]) -> Self {
        self.with_projection_mut(|proj| *proj = projection.into());
        self
    }

    pub fn predicate(mut self, predicate: Predicate) -> Self {
        self.with_predicate_mut(|pred| *pred = predicate);
        self
    }
}

impl Iterator for Iter<'_> {
    type Item = Result<RecordBatch, StorageError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.with_mut(|borrowed| match borrowed.inner.next()? {
            Ok((_, bytes)) => Some(
                RecordBatch::decompress(&borrowed.projection, &borrowed.predicate, bytes)
                    .map_err(|err| StorageError::Corrupted(err.to_string())),
            ),
            Err(err) => Some(Err(err.into())),
        })
    }
}
