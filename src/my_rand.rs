use std::ops::{Deref, DerefMut};

use nanorand::WyRand;

#[derive(Debug, Clone, Default)]
pub struct MyRand(WyRand);

impl Deref for MyRand {
    type Target = WyRand;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MyRand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
