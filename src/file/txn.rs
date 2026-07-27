use crate::error::Result;
use jammdb::Tx;

/// See [`std::collections::BTreeMap`] for comparison.
pub struct Txn<'tx> {
    pub(crate) tx: Tx<'tx>,
}

impl<'txn> Txn<'txn> {
    pub fn commit(self) -> Result<()> {
        self.tx.commit()?;
        Ok(())
    }

    pub fn rollback(self) -> Result<()> {
        drop(self);
        Ok(())
    }
}
