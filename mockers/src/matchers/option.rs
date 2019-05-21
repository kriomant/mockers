use std::marker::PhantomData;

use super::super::MatchArg;
use std::fmt::Debug;

/// Strictly speaking, this matcher is not needed, as you
/// may just use `None`, but it is added for symmetry with [`some`].
///
/// [`some`]: fn.some.html
pub fn none<T>() -> Option<T> {
    None
}

/// This struct is created by the [`some`] function. See its documentation for more.
///
/// [`some`]: fn.some.html
pub struct MatchSome<T, M: MatchArg<T>>(M, PhantomData<T>);
impl<T: Debug, M: MatchArg<T>> MatchArg<Option<T>> for MatchSome<T, M> {
    fn matches(&self, option: &Option<T>) -> Result<(), String> {
        match *option {
            Some(ref value) => self.0.matches(value),
            None => Err("is None".to_owned()),
        }
    }
    fn describe(&self) -> String {
        format!("some({})", self.0.describe())
    }
}

/// Matches a result for some value matching another matcher.
///
/// # Example
/// ```rust
/// # use mockers::MatchArg;
/// # use mockers::matchers::ANY;
/// # use mockers::matchers::in_range;
/// # use mockers::matchers::some;
///
/// let result: Option<_> = Some(42);
/// assert!(some(in_range(0..100)).matches(&result).is_ok());
/// ```
pub fn some<T, M: MatchArg<T>>(m: M) -> MatchSome<T, M> {
    MatchSome(m, PhantomData)
}
