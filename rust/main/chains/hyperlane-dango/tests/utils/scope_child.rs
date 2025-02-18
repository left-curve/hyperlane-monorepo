use std::{
    ops::{Deref, DerefMut},
    process::Child,
};

pub struct ScopeChild(Child);

impl ScopeChild {
    pub fn new(child: Child) -> Self {
        Self(child)
    }
}

impl Drop for ScopeChild {
    fn drop(&mut self) {
        self.0.kill().unwrap();
    }
}

impl Deref for ScopeChild {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ScopeChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
