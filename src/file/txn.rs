//! Transactions
//!
//! See [`BTreeMap`] for comparison.

use crate::error::Result;
use jammdb::Tx;

pub struct Txn<'tx> {
    pub(crate) tx: Tx<'tx>,
}

impl<'txn> Txn<'txn> {
    pub fn commit(self) -> Result<()> {
        self.tx.commit()?;
        Ok(())
    }
}
