use crate::MatchArg;
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait MatchArgExt<T: Debug, M: MatchArg<T>> {
    fn with_custom_msg<F: Fn(&T) -> String>(self, msg_fn: F) -> WithMessageFn<T, M, F>;
    fn with_description_fn<F: Fn() -> String>(
        self,
        description_fn: F,
    ) -> WithDescriptionFn<T, M, F>;
}

impl<T: Debug, M: MatchArg<T>> MatchArgExt<T, M> for M {
    fn with_custom_msg<F: Fn(&T) -> String>(self, msg_fn: F) -> WithMessageFn<T, M, F> {
        WithMessageFn::new(self, msg_fn)
    }
    fn with_description_fn<F: Fn() -> String>(
        self,
        description_fn: F,
    ) -> WithDescriptionFn<T, M, F> {
        WithDescriptionFn::new(self, description_fn)
    }
}

/// This struct is created by the [`with_custom_msg`] method on [`MatchArgExt`].
/// See its documentation for more.
///
/// [`MatchArgExt`]: trait.MatchArgExt.html
/// [`with_custom_msg`]: trait.MatchArgExt.html#tymethod.with_custom_msg
pub struct WithDescriptionFn<T: Debug, M: MatchArg<T>, F: Fn() -> String> {
    matcher: M,
    description_fn: F,
    _phantom: PhantomData<T>,
}
impl<T: Debug, M: MatchArg<T>, F: Fn() -> String> WithDescriptionFn<T, M, F> {
    pub fn new(matcher: M, description_fn: F) -> Self {
        WithDescriptionFn {
            matcher: matcher,
            description_fn: description_fn,
            _phantom: PhantomData,
        }
    }
    fn description(&self) -> String {
        let fun = &self.description_fn;
        fun()
    }
}
impl<T: Debug, M: MatchArg<T>, F: Fn() -> String> MatchArg<T> for WithDescriptionFn<T, M, F> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        self.matcher.matches(arg)
    }
    fn describe(&self) -> String {
        self.description()
    }
}

/// This struct is created by the [`with_description_fn`] method on [`MatchArgExt`].
/// See its documentation for more.
///
/// [`MatchArgExt`]: trait.MatchArgExt.html
/// [`with_description_fn`]: trait.MatchArgExt.html#tymethod.with_description_fn
pub struct WithMessageFn<T: Debug, M: MatchArg<T>, F: Fn(&T) -> String> {
    matcher: M,
    msg_fn: F,
    _phantom: PhantomData<T>,
}
impl<T: Debug, M: MatchArg<T>, F: Fn(&T) -> String> WithMessageFn<T, M, F> {
    pub fn new(matcher: M, msg_fn: F) -> Self {
        WithMessageFn {
            matcher: matcher,
            msg_fn: msg_fn,
            _phantom: PhantomData,
        }
    }
    fn message(&self, arg: &T) -> String {
        let fun = &self.msg_fn;
        fun(arg)
    }
}
impl<T: Debug, M: MatchArg<T>, F: Fn(&T) -> String> MatchArg<T> for WithMessageFn<T, M, F> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        match self.matcher.matches(arg) {
            Ok(()) => Ok(()),
            Err(_) => Err(self.message(arg)),
        }
    }
    fn describe(&self) -> String {
        self.matcher.describe()
    }
}
