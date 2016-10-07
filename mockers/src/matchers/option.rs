use std::marker::PhantomData;

use super::super::MatchArg;
use std::fmt::Debug;

// Strictly speaking, this matcher is not needed, as you
// may just use `None`, but it is added for symmetry.
pub fn none<T>() -> Option<T> { None }

pub struct MatchSome<T, M: MatchArg<T>>(M, PhantomData<T>);
impl<T: Debug, M: MatchArg<T>> MatchArg<Option<T>> for MatchSome<T, M> {
    fn matches(&self, option: &Option<T>) -> Result<(), String> {
        match *option {
            Some(ref value) => self.0.matches(value),
            None => Err("is None".to_owned()),
        }
    }
    fn describe(&self) -> String { format!("some({})", self.0.describe()) }
}
pub fn some<T, M: MatchArg<T>>(m: M) -> MatchSome<T, M> { MatchSome(m, PhantomData) }
