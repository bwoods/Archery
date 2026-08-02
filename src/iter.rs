use super::arrow::RecordBatch;
use super::entry::Entry;
use itertools::{PutBack, put_back};
use loom::Predicate;
use redb::{Range, ReadableTable, StorageError, Table};

impl<'a> Entry<'a> {
    pub fn iter(&'a self) -> Result<Iter<'a>, StorageError> {
        match self {
            Entry::Occupied(entry) => Iter::new(&entry.table),
            Entry::Vacant(entry) => Iter::new(&entry.table),
        }
    }
}

pub struct Iter<'a> {
    pub(crate) inner: PutBack<Range<'a, u32, &'static [u8]>>,
    pub(crate) projection: Vec<String>,
    pub(crate) predicate: Predicate,
}

impl<'a> Iter<'a> {
    pub(crate) fn new(table: &'a Table<'a, u32, &'static [u8]>) -> Result<Self, StorageError> {
        let inner = put_back(table.iter()?);
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
    type Item = Result<RecordBatch, StorageError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next()? {
            Ok((_, bytes)) => Some(
                RecordBatch::decompress(&self.projection, &self.predicate, bytes.value())
                    .map_err(|err| StorageError::Corrupted(err.to_string())),
            ),
            Err(err) => Some(Err(err)),
        }
    }
}
