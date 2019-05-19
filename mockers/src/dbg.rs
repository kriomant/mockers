use std::fmt::{Debug, Formatter, Result};

pub struct MaybeDebugWrapper<'a, T: ?Sized + DebugOnStable>(&'a T);

trait MaybeDebug {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

#[cfg(feature = "nightly")]
/// This trait is equivalent to `Debug` on non-nightly compilers, and is implemented for all types
/// on nightly compilers.
pub trait DebugOnStable {}
#[cfg(feature = "nightly")]
impl<T> DebugOnStable for T {}

#[cfg(not(feature = "nightly"))]
/// This trait is equivalent to `Debug` on non-nightly compilers, and is implemented for all types
/// on nightly compilers.
pub use std::fmt::{Debug as DebugOnStable};

#[cfg(feature = "nightly")]
impl<T> MaybeDebug for T {
    default fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "???")
    }
}

impl<T: Debug> MaybeDebug for T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.fmt(f)
    }
}

impl<'t, T: ?Sized + DebugOnStable> Debug for MaybeDebugWrapper<'t, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.fmt(f)
    }
}

pub fn dbg<T: ?Sized + DebugOnStable>(t: &T) -> MaybeDebugWrapper<'_, T> {
    MaybeDebugWrapper(t)
}
