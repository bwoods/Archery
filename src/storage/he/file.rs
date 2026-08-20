use crate::traits::deref::DerefCell;
use crate::{RecordBatch, StorageError};
use arrow_array::record_batch;
use heed::types::{Bytes, DecodeIgnore, Str};
use heed::{CompactionOption, Env, EnvFlags, EnvOpenOptions, RwTxn};
use std::path::{Path, absolute};
use tempfile::NamedTempFile;

pub struct File {
    env: DerefCell<Env>,
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

    pub fn temporary() -> Result<File, StorageError> {
        let temp = NamedTempFile::new()?;
        Self::path(temp.path())
    }

    fn env(&self) -> &Env {
        &self.env
    }

    pub fn txn(&self) -> Result<Txn<'_>, StorageError> {
        let env = self.env();

        Ok(Txn {
            txn: env.write_txn()?,
            env,
        })
    }

    pub fn page_size(&self) -> Result<u32, StorageError> {
        Ok(self.env().stat().page_size)
    }

    pub fn file_size(&self) -> Result<u64, StorageError> {
        Ok(self.env().real_disk_size()?)
    }

    pub fn file_path(&self) -> Result<String, StorageError> {
        Ok(self.env().path().to_string_lossy().to_string())
    }

    pub fn compact(&mut self) -> Result<bool, StorageError> {
        let before = self.file_size()?;
        let env = self.env.take();
        // any failures past this point leave `env` empty

        let path = env.path().to_path_buf();
        let mut temp = NamedTempFile::new()?;
        env.copy_to_file(temp.as_file_mut(), CompactionOption::Enabled)?;

        drop(env);
        temp.persist(&path)?;
        self.env = Self::path(path)?.env;

        let after = self.file_size()?;
        Ok(after != before)
    }

    pub fn stats(&self) -> Result<RecordBatch, StorageError> {
        let stats = self.env().stat();

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
    pub(crate) env: &'a Env,
    pub(crate) txn: RwTxn<'a>,
}

impl Txn<'_> {
    pub fn commit(self) -> Result<(), StorageError> {
        self.txn.commit()?;
        self.env.force_sync()?;
        Ok(())
    }

    pub fn rollback(self) -> Result<(), StorageError> {
        self.txn.abort();
        Ok(())
    }
}
