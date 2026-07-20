use crate::error::Result;
use crate::table::Table;
use arrow_array::RecordBatch;
use fallible_iterator::{FallibleIterator, IteratorExt};
use itertools::Either;
use jammdb::{Data, KVPair};
use loom::decompressors::FluxReader;
use loom::{LoomDecompressor, Predicate};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_arrow::Deserializer;

pub struct Tx<'tx> {
    tx: jammdb::Tx<'tx>,
}

#[doc(hidden)]
impl<'txn> From<jammdb::Tx<'txn>> for Tx<'txn> {
    fn from(tx: jammdb::Tx<'txn>) -> Self {
        Self { tx }
    }
}

impl<'txn> Tx<'txn> {
    pub fn get(&'txn self, id: u32) -> Result<Table<'txn>> {
        let name: [u8; 4] = id.to_be_bytes().into();
        Ok(self.tx.get_or_create_bucket(name)?.into())
    }

    pub fn iter<T: DeserializeOwned>(
        &self,
        id: u32,
        predicate: &Predicate,
    ) -> Result<impl Iterator<Item = Result<T>>> {
        let iter = self
            .batches(id, predicate)?
            .transpose_into_fallible()
            .flat_map(|batch| {
                let deserializer = Deserializer::from_record_batch(&batch)?;
                let iter = Vec::<T>::deserialize(deserializer)?
                    .into_iter()
                    .map(Ok)
                    .transpose_into_fallible();

                Ok(iter)
            })
            .iterator();

        Ok(iter)
    }

    pub fn batches(
        &self,
        id: u32,
        predicate: &Predicate,
    ) -> Result<impl Iterator<Item = Result<RecordBatch>>> {
        let iter = self.flatten(id)?.map(move |kv| {
            let reader = FluxReader::new("");
            reader
                .decompress(kv.value(), &predicate)
                .map_err(|err| err.into())
        });

        Ok(iter)
    }

    fn flatten(&self, id: u32) -> Result<impl Iterator<Item = KVPair<'_, '_>>> {
        let name: [u8; 4] = id.to_be_bytes().into();
        let table = self.tx.get_or_create_bucket(name)?;

        let iter = table.cursor().flat_map(move |data| {
            {
                match data {
                    Data::KeyValue(kv) => {
                        // ⑴ Table stored, in its entirety, in this bucket
                        Either::Left(Some(kv).into_iter())
                    }
                    Data::Bucket(name) => {
                        let table = table.get_bucket(name).expect("?");
                        Either::Right(table.cursor().flat_map(move |data| {
                            match data {
                                // ⑵ Table is broken up into chunks; each stored in a sub-bucket
                                Data::KeyValue(kv) => Either::Left(Some(kv).into_iter()),
                                Data::Bucket(name) => {
                                    let table = table.get_bucket(name).expect("??");
                                    Either::Right(table.cursor().map(|data| {
                                        match data {
                                            // ⑶ stored, one row at a time, each stored in a sub-sub-bucket
                                            Data::KeyValue(kv) => kv,
                                            Data::Bucket(_) => unreachable!(), // buckets may only nest this far!
                                        }
                                    }))
                                }
                            }
                        }))
                    }
                }
            }
        });

        Ok(iter)
    }
}
