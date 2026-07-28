use crate::entry::OccupiedEntry;
use jammdb::{Cursor, Data, KVPair};
pub use map_into::MapInto;
use serde::de::DeserializeOwned;

mod map_into;

pub struct Iter<'a, K> {
    pub(crate) outer: Cursor<'a, 'a>,
    pub(crate) inner: Option<Cursor<'a, 'a>>,
    pub(crate) occupied: OccupiedEntry<'a, K>,
}

impl<'a, K> Iterator for Iter<'a, K> {
    type Item = KVPair<'a, 'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            Some(ref mut inner) => match inner.next() {
                Some(data) => match data {
                    Data::KeyValue(kv) => Some(kv),
                    Data::Bucket(_) => unreachable!(),
                },
                None => {
                    self.inner = None;
                    self.next() // recurse; grab the next outer
                }
            },
            None => match self.outer.next() {
                Some(data) => match data {
                    Data::KeyValue(kv) => Some(kv),
                    Data::Bucket(name) => {
                        let bucket = self.occupied.slot.parent.get_bucket(name).unwrap();
                        self.inner = Some(bucket.cursor());
                        self.next() // recurse; grab the next inner
                    }
                },
                None => None,
            },
        }
    }
}

impl<'a, K> Iter<'a, K> {
    pub fn map_into<T: DeserializeOwned>(self) -> MapInto<'a, K, T> {
        MapInto {
            outer: self,
            queue: Default::default(),
        }
    }
}
