use std::marker::PhantomData;

use super::super::MatchArg;
use std::fmt::Debug;

/// This struct is created by the [`ok`] function. See its documentation for more.
///
/// [`ok`]: fn.ok.html
pub struct MatchOk<T, M: MatchArg<T>>(M, PhantomData<T>);
impl<T: Debug, E: Debug, M: MatchArg<T>> MatchArg<Result<T, E>> for MatchOk<T, M> {
    fn matches(&self, result: &Result<T, E>) -> Result<(), String> {
        match *result {
            Ok(ref value) => self.0.matches(value),
            Err(..) => Err(format!("{:?} is not Ok", result)),
        }
    }
    fn describe(&self) -> String {
        format!("ok({})", self.0.describe())
    }
}

/// Matches a result for a successful value matching another matcher.
///
/// # Example
/// ```rust
/// # use mockers::MatchArg;
/// # use mockers::matchers::ANY;
/// # use mockers::matchers::in_range;
/// # use mockers::matchers::ok;
///
/// let result: Result<_, ()> = Ok(42);
/// assert!(ok(in_range(0..100)).matches(&result).is_ok());
/// ```
pub fn ok<T, M: MatchArg<T>>(m: M) -> MatchOk<T, M> {
    MatchOk(m, PhantomData)
}

/// This struct is created by the [`err`] function. See its documentation for more.
///
/// [`err`]: fn.err.html
pub struct MatchErr<E, M: MatchArg<E>>(M, PhantomData<E>);
impl<E: Debug, T: Debug, M: MatchArg<E>> MatchArg<Result<T, E>> for MatchErr<E, M> {
    fn matches(&self, result: &Result<T, E>) -> Result<(), String> {
        match *result {
            Err(ref err) => self.0.matches(err),
            Ok(..) => Err(format!("{:?} is not Err", result)),
        }
    }
    fn describe(&self) -> String {
        format!("err({})", self.0.describe())
    }
}

/// Matches a result for an error matching another matcher.
///
/// # Example
/// ```rust
/// # use mockers::MatchArg;
/// # use mockers::matchers::ANY;
/// # use mockers::matchers::err;
///
/// let result: Result<(), _> = Err("Some error");
/// assert!(err(ANY).matches(&result).is_ok());
/// ```
pub fn err<E, M: MatchArg<E>>(m: M) -> MatchErr<E, M> {
    MatchErr(m, PhantomData)
}
