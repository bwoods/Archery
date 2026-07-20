use crate::error::Error;
use crate::txn::Tx;
use std::path::Path;

struct DB {
    db: jammdb::DB,
}

impl DB {
    pub fn file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let db = jammdb::OpenOptions::new()
            .pagesize(4096) // must be ≥ 1024
            .num_pages(4) // “this function will panic if you provide a value < 4”
            .open(path.as_ref())?;

        Ok(Self { db })
    }

    pub fn tx(&self, writable: bool) -> Result<Tx<'_>, Error> {
        Ok(self.db.tx(true)?.into())
    }
}

#[cfg(test)]
mod tests {
    use jammdb::*;

    fn temp_path() -> Result<std::path::PathBuf, std::io::Error> {
        let file = tempfile::NamedTempFile::new()?;
        let path = file.path().to_path_buf();

        file.close()?; // remove the file; jammdb fails if the (empty) file exists
        Ok(path) // but we’ll (re)use the path
    }

    #[test]
    fn text_storage() -> Result<(), Error> {
        let db = OpenOptions::new()
            .pagesize(4096)
            .num_pages(4) // “this function will panic if you provide a value < 4”
            .open(temp_path()?)?;
        let tx = db.tx(true)?;

        let name = 1u32.to_be_bytes();
        let _table = tx.get_or_create_bucket(name);

        tx.commit()
    }
}
