use std::num::NonZeroU64;

pub type Id = NonZeroU64;

pub trait Entity {
    fn id(&self) -> u32;
    fn generation(&self) -> u32;

    fn from(id: u32, generation: u32) -> Self;
}

impl Entity for NonZeroU64 {
    fn id(&self) -> u32 {
        self.get() as u32
    }

    fn generation(&self) -> u32 {
        (self.get() >> 32) as u32
    }

    fn from(id: u32, generation: u32) -> Self {
        Self::new((generation as u64) << 32 | id as u64).unwrap()
    }
}
