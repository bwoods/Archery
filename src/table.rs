use crate::error::Error;
use arrow_array::RecordBatch;
use itertools::Either;
use jammdb::{Bucket, Data};
use loom::decompressors::FluxReader;
use loom::{LoomDecompressor, Predicate};

pub struct Table<'txn> {
    table: Bucket<'txn, 'txn>,
}

#[doc(hidden)]
impl<'txn> From<Bucket<'txn, 'txn>> for Table<'txn> {
    fn from(table: Bucket<'txn, 'txn>) -> Self {
        Self { table }
    }
}

impl Table<'_> {
    fn iter(&self) -> impl Iterator<Item = Result<RecordBatch, Error>> {
        self.table
            .cursor()
            .flat_map(|data| {
                match data {
                    Data::KeyValue(kv) => {
                        // Table stored, in its entirety, in this bucket
                        Either::Left(Some(kv).into_iter())
                    }
                    Data::Bucket(name) => {
                        // Table is broken up into chunks; each stored in sub-buckets
                        Either::Right(
                            self.table
                                .get_bucket(name)
                                .expect("bucket") // jammdb gave us a non-existent bucket name?
                                .cursor()
                                .map(|data| {
                                    match data {
                                        Data::KeyValue(kv) => kv,
                                        Data::Bucket(_) => unreachable!(), // buckets may only nest once
                                    }
                                }),
                        )
                    }
                }
            })
            .map(|kv| {
                let reader = FluxReader::default();
                match reader.decompress(kv.value(), &Predicate::None) {
                    Ok(batch) => Ok(batch),
                    Err(err) => Err(err.into()),
                }
            })
    }
}
