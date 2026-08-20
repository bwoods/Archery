use std::cell::OnceCell;
use std::ops::{Deref, DerefMut};

pub struct DerefCell<T> {
    inner: OnceCell<T>,
}

impl<T> Deref for DerefCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.get().unwrap()
    }
}

impl<T> DerefMut for DerefCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.get_mut().unwrap()
    }
}

impl<T> From<T> for DerefCell<T> {
    fn from(value: T) -> Self {
        DerefCell {
            inner: value.into(),
        }
    }
}

impl<T> From<Option<T>> for DerefCell<T> {
    fn from(value: Option<T>) -> Self {
        let inner = OnceCell::new();

        if let Some(value) = value {
            inner.set(value).ok();
        };

        DerefCell { inner }
    }
}

impl<T> DerefCell<T> {
    pub fn get(&mut self) -> Option<T> {
        self.inner.take()
    }

    pub fn take(&mut self) -> T {
        self.inner.take().unwrap()
    }
}
