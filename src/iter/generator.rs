use crate::entry::occupied::get;
use crate::error::{Error, Result};
use arrow_array::RecordBatch;
use arrow_schema::{Schema, SchemaRef};
use fallible_iterator::{FallibleIterator, IteratorExt};
use jammdb::{Bucket, Cursor, Data, KVPair, ToBuckets};
use loom::Predicate;
use yield_return::{LocalIter, LocalIterContext};

pub(crate) fn schema<'a>(parent: &Bucket<'a, 'a>, key: impl AsRef<[u8]>) -> Result<SchemaRef> {
    let schema = flatten(parent, key, &Predicate::None, &[])
        .map(|batch| Ok(batch.schema()))
        .next()?
        .unwrap_or_else(|| SchemaRef::new(Schema::empty()));

    Ok(schema)
}

pub(crate) fn flat_map<'a, F, I, T>(
    parent: &Bucket<'a, 'a>,
    key: impl AsRef<[u8]>,
    predicate: &Predicate,
    projection: &[String],
    mut f: F,
) -> impl FallibleIterator<Item = T, Error = Error>
where
    F: FnMut(RecordBatch) -> Result<I>,
    I: IntoIterator<Item = T>,
{
    flatten(parent, key, predicate, projection).flat_map(move |batch| {
        Ok(f(batch)?
            .into_iter()
            .map(Ok) // needed for transpose_into_fallible
            .transpose_into_fallible())
    })
}

pub(crate) fn flatten<'a>(
    parent: &Bucket<'a, 'a>,
    key: impl AsRef<[u8]>,
    predicate: &Predicate,
    projection: &[String],
) -> impl FallibleIterator<Item = RecordBatch, Error = Error> {
    let mut cursor = parent.cursor();
    cursor.seek(key.as_ref());

    generator(cursor)
        .map(move |kv| get(kv.value(), predicate, projection))
        .transpose_into_fallible()
}

fn generator<'a>(cursor: Cursor<'a, 'a>) -> impl Iterator<Item = KVPair<'a, 'a>> {
    LocalIter::new(|mut then| async move { recursive(cursor, &mut then).await })
}

async fn recursive<'a>(cursor: Cursor<'a, 'a>, then: &mut LocalIterContext<KVPair<'a, 'a>>) {
    match cursor.current() {
        None => return,
        Some(Data::KeyValue(kv)) => then.ret(kv).await,
        Some(Data::Bucket(_)) => {
            for (_, bucket) in cursor.to_buckets() {
                // “error[E0733]: recursion in an async fn requires boxing”
                Box::pin(recursive(bucket.cursor(), then)).await;
            }
        }
    }
}
