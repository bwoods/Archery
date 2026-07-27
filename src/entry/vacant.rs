use crate::Compression;
use crate::entry::occupied::Occupied;
use crate::error::Result;
use arrow_array::RecordBatch;
use jammdb::{Bucket, ToBytes};
use loom::{LoomCompressor, compressors::FluxWriter};

pub struct Vacant<'a, K> {
    pub(crate) compression: Compression,
    pub(crate) parent: Bucket<'a, 'a>,
    pub(crate) key: K,
}

impl<'a, K: ToBytes<'a> + Clone> Vacant<'a, K> {
    pub fn with_profile(self, profile: Compression) -> Self {
        Self {
            compression: profile,
            ..self
        }
    }

    pub fn key(&self) -> K {
        self.key.clone()
    }

    #[inline]
    pub fn insert(self, value: RecordBatch) -> Result<Occupied<'a, K>> {
        insert_entry(self.parent, self.key.clone(), self.compression, value)
    }
}

pub(crate) fn insert_entry<'a, K>(
    parent: Bucket<'a, 'a>,
    key: K,
    compression: Compression,
    value: RecordBatch,
) -> Result<Occupied<'a, K>>
where
    K: ToBytes<'a> + Clone,
{
    insert(&parent, &key, compression, value)?;

    Ok(Occupied {
        compression,
        parent,
        key,
    })
}

pub(crate) fn insert<'a, K>(
    parent: &Bucket<'a, 'a>,
    key: &K,
    compression: Compression,
    value: RecordBatch,
) -> Result<()>
where
    K: ToBytes<'a> + Clone,
{
    let writer = FluxWriter::with_profile(compression.into()).with_u64_only(true);
    parent.put(key.clone(), writer.compress(&value)?)?;

    Ok(())
}
