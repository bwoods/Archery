use super::entry::{Entry, OccupiedEntry};
use crate::arrow::RecordBatch;
use itertools::{PutBack, put_back};
use jammdb::{Cursor, Data, Error};
use loom::Predicate;

impl<'a> Entry<'a> {
    pub fn iter(&'a self) -> Result<Iter<'a>, Error> {
        match self {
            Entry::Occupied(entry) => Iter::new(&entry.table),
            Entry::Vacant(entry) => Iter::new(&entry.table),
        }
    }
}

pub struct Iter<'a> {
    pub(crate) inner: PutBack<Cursor<'a, 'a>>,
    pub(crate) projection: Vec<String>,
    pub(crate) predicate: Predicate,
}

impl<'a> Iter<'a> {
    pub(crate) fn new(entry: &'a OccupiedEntry<'a>) -> Result<Iter<'a>, Error> {
        let tx = entry.db.tx(true)?;
        let table = tx.get_bucket(entry.key())?;

        let inner = put_back(table.cursor());
        let projection = Vec::default();
        let predicate = Predicate::None;

        Ok(Iter {
            inner,
            projection,
            predicate,
        })
    }

    pub fn projection(self, projection: &[String]) -> Self {
        Iter {
            projection: projection.into(),
            ..self
        }
    }

    pub fn predicate(self, predicate: Predicate) -> Self {
        Iter { predicate, ..self }
    }
}

impl Iterator for Iter<'_> {
    type Item = Result<RecordBatch, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next()? {
            Data::KeyValue(kv) => Some(
                RecordBatch::decompress(&self.projection, &self.predicate, kv.value())
                    .map_err(|err| Error::InvalidDB(err.to_string())),
            ),
            Data::Bucket(_) => Some(Err(Error::IncompatibleValue)),
        }
    }
}
