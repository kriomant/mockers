
use std::marker::PhantomData;
use MatchArg;

pub trait MatchArgExt<T, M: MatchArg<T>> {
    fn with_description_fn<F: Fn() -> String>(self, description_fn: F) -> WithDescriptionFn<T, M, F>;
}

impl<T, M: MatchArg<T>> MatchArgExt<T, M> for M {
    fn with_description_fn<F: Fn() -> String>(self, description_fn: F) -> WithDescriptionFn<T, M, F> {
        WithDescriptionFn::new(self, description_fn)
    }
}


pub struct WithDescriptionFn<T, M: MatchArg<T>, F: Fn() -> String> {
    matcher: M,
    description_fn: F,
    _phantom: PhantomData<T>,
}
impl<T, M: MatchArg<T>, F: Fn() -> String> WithDescriptionFn<T, M, F> {
    pub fn new(matcher: M, description_fn: F) -> Self {
        WithDescriptionFn {
            matcher: matcher,
            description_fn: description_fn,
            _phantom: PhantomData,
        }
    }
}
impl<T, M: MatchArg<T>, F: Fn() -> String> MatchArg<T> for WithDescriptionFn<T, M, F> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        self.matcher.matches(arg)
    }
    fn describe(&self) -> String {
        let fun = &self.description_fn;
        fun()
    }
}
