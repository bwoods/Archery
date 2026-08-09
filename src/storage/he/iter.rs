use super::entry::{Entry, Table};
use crate::{RecordBatch, StorageError};
use heed::byteorder::BigEndian;
use heed::types::{Bytes, U32};
use heed::{RoIter, RwTxn};
use itertools::{PutBack, put_back};
use loom::Predicate;
use ouroboros::self_referencing;

impl<'a> Entry<'a> {
    pub fn iter(&'a self) -> Result<Iter<'a>, StorageError> {
        match self {
            Entry::Occupied(entry) => Iter::new(entry.table, entry.txn.as_ref().unwrap()),
            Entry::Vacant(entry) => Iter::new(entry.table, entry.txn.as_ref().unwrap()),
        }
    }
}

#[self_referencing]
pub struct Inner<'a> {
    pub(crate) txn: &'a RwTxn<'a>,
    #[borrows(mut txn)]
    #[not_covariant]
    pub(crate) iter: PutBack<RoIter<'this, U32<BigEndian>, Bytes>>,
}

pub struct Iter<'a> {
    pub(crate) inner: Inner<'a>,
    pub(crate) projection: Vec<String>,
    pub(crate) predicate: Predicate,
}

impl<'a> Iter<'a> {
    pub(crate) fn new(table: Table, txn: &'a RwTxn<'a>) -> Result<Self, StorageError> {
        let inner = InnerTryBuilder {
            txn,
            iter_builder: |txn| {
                let iter: Result<_, StorageError> = Ok(put_back(table.iter(txn)?));
                iter
            },
        }
        .try_build()?;

        Ok(Self {
            inner,
            projection: Vec::new(),
            predicate: Predicate::None,
        })
    }

    pub fn projection(mut self, projection: &[String]) -> Self {
        self.projection = projection.into();
        self
    }

    pub fn predicate(mut self, predicate: Predicate) -> Self {
        self.predicate = predicate;
        self
    }
}

impl Iterator for Iter<'_> {
    type Item = Result<RecordBatch, StorageError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.with_iter_mut(|iter| iter.next())? {
            Ok((_, bytes)) => Some(
                RecordBatch::decompress(&self.projection, &self.predicate, bytes)
                    .map_err(|err| StorageError::Corrupted(err.to_string())),
            ),
            Err(err) => Some(Err(err.into())),
        }
    }
}
