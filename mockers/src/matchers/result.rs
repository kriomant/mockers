use std::marker::PhantomData;

use super::super::MatchArg;
use std::fmt::Debug;

pub struct MatchOk<T, M: MatchArg<T>>(M, PhantomData<T>);
impl<T: Debug, E: Debug, M: MatchArg<T>> MatchArg<Result<T, E>> for MatchOk<T, M> {
    fn matches(&self, result: &Result<T, E>) -> Result<(), String> {
        match *result {
            Ok(ref value) => self.0.matches(value),
            Err(..) => Err(format!("{:?} is not Ok", result)),
        }
    }
    fn describe(&self) -> String { format!("ok({})", self.0.describe()) }
}
pub fn ok<T, M: MatchArg<T>>(m: M) -> MatchOk<T, M> { MatchOk(m, PhantomData) }

pub struct MatchErr<E, M: MatchArg<E>>(M, PhantomData<E>);
impl<E: Debug, T: Debug, M: MatchArg<E>> MatchArg<Result<T, E>> for MatchErr<E, M> {
    fn matches(&self, result: &Result<T, E>) -> Result<(), String> {
        match *result {
            Err(ref err) => self.0.matches(err),
            Ok(..) => Err(format!("{:?} is not Err", result)),
        }
    }
    fn describe(&self) -> String { format!("err({})", self.0.describe()) }
}
pub fn err<E, M: MatchArg<E>>(m: M) -> MatchErr<E, M> { MatchErr(m, PhantomData) }
