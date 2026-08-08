use super::entry::{Entry, OccupiedEntry, VacantEntry};
use crate::{RecordBatch, StorageError};
use arrow_array::record_batch;
use heed::{Env, EnvFlags, EnvOpenOptions, RwTxn};
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
                .flags(EnvFlags::NO_SUB_DIR)
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
            env: self.env.clone(),
        })
    }

    pub fn compact(&mut self) -> Result<bool, StorageError> {
        Ok(false)
    }

    pub fn stats(&self) -> Result<RecordBatch, StorageError> {
        let stats = self.env.stat();

        let batch = record_batch!(
            ("tree height", UInt64, [stats.depth as u64]),
            ("tree entries", UInt64, [stats.entries as u64]),
            ("branch pages", UInt64, [stats.branch_pages as u64]),
            ("leaf pages", UInt64, [stats.leaf_pages as u64]),
            ("overflow pages", UInt64, [stats.overflow_pages as u64]),
            ("page size", UInt64, [stats.page_size as u64])
        )?;

        Ok(batch.into())
    }
}

pub struct Txn<'a> {
    env: Rc<Env>,
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
