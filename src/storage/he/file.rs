use super::entry::{Entry, OccupiedEntry, VacantEntry};
use crate::{RecordBatch, StorageError};
use arrow_array::record_batch;
use heed::types::{Bytes, DecodeIgnore, Str};
use heed::{Database, Env, EnvFlags, EnvOpenOptions, RwTxn};
use std::path::{Path, absolute};
use std::rc::Rc;
use tempfile::NamedTempFile;

pub struct File {
    env: Rc<Env>,
}

impl File {
    pub fn path(path: impl AsRef<Path>) -> Result<File, StorageError> {
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(1024 * 1024 * 1024)
                .max_dbs(512)
                .flags(EnvFlags::NO_SUB_DIR | EnvFlags::NO_LOCK)
                .open(absolute(path.as_ref())?)?
        };
        Ok(Self { env: Rc::new(env) })
    }

    pub fn temporary() -> Result<File, StorageError> {
        let temp = NamedTempFile::new()?;
        Self::path(temp.path())
    }

    pub fn txn(&self) -> Result<Txn<'_>, StorageError> {
        Ok(Txn {
            txn: self.env.write_txn()?,
            env: &self.env,
        })
    }

    pub fn compact(&mut self) -> Result<bool, StorageError> {
        Ok(false)
    }

    pub fn stats(&self) -> Result<RecordBatch, StorageError> {
        let stats = self.env.stat();

        let mut batch: RecordBatch = record_batch!(
            ("tree height", Utf8, [stats.depth.to_string()]),
            ("tree entries", Utf8, [stats.entries.to_string()]),
            ("branch pages", Utf8, [stats.branch_pages.to_string()]),
            ("leaf pages", Utf8, [stats.leaf_pages.to_string()]),
            ("overflow pages", Utf8, [stats.overflow_pages.to_string()]),
            ("", Utf8, [""])
        )?
        .into();

        let txn = self.env.read_txn()?;
        let all: Database<Str, DecodeIgnore> = self.env.open_database(&txn, None)?.unwrap();

        let mut batches = Vec::new();
        for db in all.iter(&txn)? {
            let (name, ()) = db?;

            if let Ok(Some(db)) = self.env.open_database::<Str, Bytes>(&txn, Some(name)) {
                let stats = db.stat(&txn)?;
                batches.push(
                    record_batch!(
                        ("tree height", Utf8, [stats.depth.to_string()]),
                        ("tree entries", Utf8, [stats.entries.to_string()]),
                        ("branch pages", Utf8, [stats.branch_pages.to_string()]),
                        ("leaf pages", Utf8, [stats.leaf_pages.to_string()]),
                        ("overflow pages", Utf8, [stats.overflow_pages.to_string()]),
                        ("", Utf8, [name])
                    )?
                    .into(),
                );
            }
        }

        txn.commit()?;

        batch.extend(batches.iter());
        Ok(batch)
    }
}

pub struct Txn<'a> {
    env: &'a Env,
    txn: RwTxn<'a>,
}

impl<'a> Txn<'a> {
    pub fn commit(self) -> Result<(), StorageError> {
        self.txn.commit()?;
        self.env.force_sync()?;
        Ok(())
    }

    pub fn rollback(self) -> Result<(), StorageError> {
        self.txn.abort();
        Ok(())
    }

    pub fn entry(&mut self, name: &str) -> Result<Entry<'_>, StorageError> {
        let table = self.env.create_database(&mut self.txn, Some(name))?;

        let entry = match table.len(&mut self.txn)? > 0 {
            false => Entry::Vacant(VacantEntry {
                table,
                txn: Some(self.env.nested_write_txn(&mut self.txn)?),
                key: name.to_string(),
            }),
            true => Entry::Occupied(OccupiedEntry {
                table,
                txn: Some(self.env.nested_write_txn(&mut self.txn)?),
                key: name.to_string(),
            }),
        };

        Ok(entry)
    }
}
