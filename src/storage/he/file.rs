use super::entry::{Entry, OccupiedEntry, VacantEntry};
use crate::{RecordBatch, StorageError};
use arrow_array::record_batch;
use heed::types::{Bytes, DecodeIgnore, Str};
use heed::{CompactionOption, Env, EnvFlags, EnvOpenOptions, RwTxn};
use std::path::{Path, absolute};
use tempfile::NamedTempFile;

pub struct File {
    env: Option<Env>,
}

impl File {
    pub fn path(path: impl AsRef<Path>) -> Result<File, StorageError> {
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(1024 * 1024 * 1024)
                .max_dbs(512)
                .flags(EnvFlags::NO_SUB_DIR | EnvFlags::NO_LOCK)
                .open(absolute(path.as_ref())?)?
                .into()
        };

        Ok(Self { env })
    }

    fn env(&self) -> &Env {
        self.env.as_ref().unwrap() // only removed during compaction
    }

    pub fn temporary() -> Result<File, StorageError> {
        let temp = NamedTempFile::new()?;
        Self::path(temp.path())
    }

    pub fn txn(&self) -> Result<Txn<'_>, StorageError> {
        let env = self.env();

        Ok(Txn {
            txn: env.write_txn()?,
            env,
        })
    }

    pub fn compact(&mut self) -> Result<bool, StorageError> {
        let env = self.env.take().expect("File::env");
        // any failures past this point leave `env` empty

        let path = env.path().to_path_buf();
        let mut temp = NamedTempFile::new()?;
        env.copy_to_file(temp.as_file_mut(), CompactionOption::Enabled)?;

        drop(env);
        temp.persist(&path)?;
        let file = Self::path(path)?;

        let File { env } = file;
        self.env = env;

        Ok(true)
    }

    pub fn stats(&self) -> Result<RecordBatch, StorageError> {
        let stats = self.env().stat();

        println!("page size: {}", stats.page_size); // TODO: log-level
        let mut batch: RecordBatch = record_batch!(
            ("entries", UInt64, [stats.entries as u64]),
            ("height", UInt32, [stats.depth]),
            ("branches", UInt64, [stats.branch_pages as u64]),
            ("leaves", UInt64, [stats.leaf_pages as u64]),
            ("overflow", UInt64, [stats.overflow_pages as u64]),
            ("", Utf8, [""])
        )?
        .into();

        let mut batches = Vec::new();
        let txn = self.env().read_txn()?;

        if let Some(all) = self.env().open_database::<Str, DecodeIgnore>(&txn, None)? {
            for db in all.iter(&txn)? {
                let (name, ()) = db?;

                if let Some(db) = self.env().open_database::<Str, Bytes>(&txn, Some(name))? {
                    let stats = db.stat(&txn)?;
                    batches.push(
                        record_batch!(
                            ("entries", UInt64, [stats.entries as u64]),
                            ("height", UInt32, [stats.depth]),
                            ("branches", UInt64, [stats.branch_pages as u64]),
                            ("leaves", UInt64, [stats.leaf_pages as u64]),
                            ("overflow", UInt64, [stats.overflow_pages as u64]),
                            ("", Utf8, [name])
                        )?
                        .into(),
                    );
                }
            }
        }

        txn.commit()?; // keeps LMDB happy in multi-process situations

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
