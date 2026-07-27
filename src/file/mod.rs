use crate::error::Result;
use crate::file::txn::Txn;
use jammdb::DB;
use std::path::Path;

mod temp;
pub mod txn;

pub struct File {
    db: DB,
}

impl File {
    pub fn file(path: impl AsRef<Path>) -> Result<Self> {
        let db = jammdb::OpenOptions::new()
            .pagesize(4096) // must be ≥ 1024
            .num_pages(4) // “this function will panic if you provide a value < 4”
            .open(path.as_ref())?;

        Ok(Self { db })
    }

    pub fn txn(&self) -> Result<Txn<'_>> {
        let tx = self.db.tx(true)?;
        Ok(Txn { tx })
    }
}
