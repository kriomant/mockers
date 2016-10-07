
use std::marker::PhantomData;
use std::fmt::Debug;
use MatchArg;

pub trait MatchArgExt<T: Debug, M: MatchArg<T>> {
    fn with_custom_msg<F: Fn(&T) -> String>(self, msg_fn: F) -> WithMessageFn<T, M, F>;
    fn with_description_fn<F: Fn() -> String>(self, description_fn: F) -> WithDescriptionFn<T, M, F>;
}

impl<T: Debug, M: MatchArg<T>> MatchArgExt<T, M> for M {
    fn with_custom_msg<F: Fn(&T) -> String>(self, msg_fn: F) -> WithMessageFn<T, M, F> {
        WithMessageFn::new(self, msg_fn)
    }
    fn with_description_fn<F: Fn() -> String>(self, description_fn: F) -> WithDescriptionFn<T, M, F> {
        WithDescriptionFn::new(self, description_fn)
    }
}


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
