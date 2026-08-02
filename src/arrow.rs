use crate::Compression;
use arrow_schema::Schema;
use arrow_select::{concat::concat_batches, filter::filter_record_batch};
use loom::{FluxError, LoomCompressor, LoomDecompressor, Predicate};
use loom::{compressors::FluxWriter, decompressors::FluxReader};
use std::borrow::Borrow;
use std::iter::once;
use std::ops::Deref;
use std::sync::Arc;

pub struct RecordBatch(pub(crate) arrow_array::RecordBatch);

impl RecordBatch {
    pub(crate) fn compress(
        compression: Compression,
        batch: &RecordBatch,
    ) -> Result<impl AsRef<[u8]>, FluxError> {
        let writer = FluxWriter::with_profile(compression.into()).with_u64_only(true);
        writer.compress(batch)
    }

    pub(crate) fn decompress(
        projection: &[String],
        predicate: &Predicate,
        bytes: impl AsRef<[u8]>,
    ) -> Result<RecordBatch, FluxError> {
        let reader = FluxReader::new("");
        if projection.is_empty() {
            reader.decompress(bytes.as_ref(), &predicate)
        } else {
            reader.decompress_projected(bytes.as_ref(), &predicate, &projection)
        }
        .and_then(|batch| {
            if matches!(predicate, Predicate::None) {
                Ok(batch)
            } else {
                let mask = predicate.eval_on_batch(&batch)?;
                filter_record_batch(&batch, &mask).map_err(|err| err.into())
            }
        })
        .map(RecordBatch)
    }
}

impl AsRef<<Self as Deref>::Target> for RecordBatch {
    #[inline]
    fn as_ref(&self) -> &<Self as Deref>::Target {
        self
    }
}

impl Borrow<<Self as Deref>::Target> for RecordBatch {
    #[inline]
    fn borrow(&self) -> &<Self as Deref>::Target {
        self
    }
}

impl Deref for RecordBatch {
    type Target = arrow_array::RecordBatch;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for RecordBatch {
    #[inline]
    fn default() -> Self {
        let schema = Arc::new(Schema::empty());
        RecordBatch(arrow_array::RecordBatch::new_empty(schema))
    }
}

impl<'a> Extend<&'a RecordBatch> for RecordBatch {
    /// # Panics
    /// Panics if the `RecordBatch`s within `I` do not have a matching schema.
    #[track_caller]
    fn extend<I: IntoIterator<Item = &'a RecordBatch>>(&mut self, iter: I) {
        let schema = self.schema();
        self.0 = concat_batches(
            &schema, //
            once(&self.0).chain(iter.into_iter().map(|x| &x.0)),
        )
        .unwrap();
    }
}

#[test]
fn extend_compiles() {
    let mut a = RecordBatch::default();
    let b = RecordBatch::default();

    a.extend(&[b]);
}
