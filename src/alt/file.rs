use crate::alt::entry::{Entry, OccupiedEntry, VacantEntry};
use jammdb::{DB, Error};
use std::path::Path;
use std::sync::Arc;

pub struct File {
    db: Arc<DB>,
}

impl File {
    pub fn path(path: impl AsRef<Path>) -> Result<File, Error> {
        let db = Arc::new(DB::open(&path)?);
        Ok(Self { db })
    }

    pub fn txn(&self) -> Result<Txn, Error> {
        let db = self.db.clone();
        Ok(Txn { db })
    }

    pub fn compact(&mut self) -> Result<bool, Error> {
        Ok(false)
    }
}

pub struct Txn {
    db: Arc<DB>,
}

impl Txn {
    pub fn commit(self) -> Result<(), Error> {
        Ok(())
    }

    pub fn rollback(self) -> Result<(), Error> {
        Ok(())
    }

    pub fn entry(&self, name: String) -> Result<Entry<'_>, Error> {
        let tx = self.db.tx(false)?;
        let table = tx.get_bucket(name.clone());

        let entry = match tx.get_bucket(name.clone()) {
            Ok(table) => Entry::Occupied(OccupiedEntry {
                db: self.db.clone(),
                key: name,
                _marker: Default::default(),
            }),
            Err(Error::BucketMissing) => Entry::Vacant(VacantEntry {
                db: self.db.clone(),
                key: name,
                _marker: Default::default(),
            }),
            Err(err) => return Err(err),
        };

        Ok(entry)
    }
}

// pub struct Txn<'a> {
//     txn: Tx<'a>,
// }
//
// impl Txn<'_> {
//     pub fn commit(self) -> Result<(), Error> {
//         self.txn.commit()?;
//         Ok(())
//     }
//
//     pub fn rollback(self) -> Result<(), Error> {
//         drop(self);
//         Ok(())
//     }
//
//     pub fn entry<'b>(&'b self, name: String) -> Result<Entry<'b>, Error> {
//         let table = self.txn.get_or_create_bucket(name.clone())?;
//
//         let entry = match table.next_int() == 0 {
//             true => Entry::Vacant(VacantEntry { table, key: name }),
//             false => Entry::Occupied(OccupiedEntry { table, key: name }),
//         };
//
//         Ok(entry)
//     }
// }
