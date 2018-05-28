use std::fmt::{Debug, Formatter, Result};

pub struct MaybeDebugWrapper<'a, T: 'a + ?Sized>(&'a T);

trait MaybeDebug {
    fn fmt(&self, f: &mut Formatter) -> Result;
}
impl<T> MaybeDebug for T {
    default fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "???")
    }
}
impl<T: Debug> MaybeDebug for T {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.fmt(f)
    }
}

impl<'t, T: ?Sized> Debug for MaybeDebugWrapper<'t, T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.0.fmt(f)
    }
}

pub fn dbg<T: ?Sized>(t: &T) -> MaybeDebugWrapper<T> {
    MaybeDebugWrapper(t)
}
