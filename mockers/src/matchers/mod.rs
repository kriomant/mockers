use super::MatchArg;

use std::fmt::Debug;
use std::marker::PhantomData;

use std;
use std::collections::Bound;
use std::fmt::Write;
use std::ops::RangeBounds;

pub use self::ext::*;
pub use self::option::*;
pub use self::result::*;

mod ext;
mod option;
mod result;

/// Type of [`ANY`]. See its documentation for more.
///
/// [`ANY`]: constant.ANY.html
pub struct MatchAny;
impl ToString for MatchAny {
    fn to_string(&self) -> String {
        "_".to_owned()
    }
}
impl<T> MatchArg<T> for MatchAny {
    fn matches(&self, _: &T) -> Result<(), String> {
        Ok(())
    }

    fn describe(&self) -> String {
        "_".to_owned()
    }
}

/// Matches any value. You may also use [`any`] when matching parameters with generic type.
///
/// [`any`]: fn.any.html
pub const ANY: MatchAny = MatchAny;

/// This struct is created by the [`any`] function. See its documentation for more.
///
/// [`any`]: fn.any.html
pub struct MatchAnyT<T>(PhantomData<T>);
impl<T> MatchArg<T> for MatchAnyT<T> {
    fn matches(&self, _: &T) -> Result<(), String> {
        Ok(())
    }

    fn describe(&self) -> String {
        "_".to_owned()
    }
}

/// Matches any value. Must be used instead of [`ANY`] if you are using generics.
///
/// [`ANY`]: constant.ANY.html
pub fn any<T>() -> MatchAnyT<T> {
    MatchAnyT(PhantomData)
}

/// Hack for interpreting macro result as token stream and
/// converting it to items.
/// It is used to overcome inability to properly receive
/// type bounds as macro parameter.
/// Stolen from http://stackoverflow.com/a/30293051
macro_rules! to_items {
    ($($item:item)*) => ($($item)*);
}

/// Generate matcher for comparison operator.
///
/// Example of code generated for
/// `simple_matcher!(lt, LtMatchArg,  <, "not less than", PartialOrd);`:
/// ```
/// pub struct LtMatchArg<T>(T);
/// impl<T: PartialOrd + std::fmt::Debug> MatchArg<T> for LtMatchArg<T> {
///     fn matches(&self, arg: &T) -> Result<(), String> {
///         if *arg < self.0 {
///             Ok(())
///         } else {
///             Err(format!("{:?} is not less than {:?}", arg, self.0))
///          }
///     }
///
///     fn describe(&self) -> String {
///         format!("lt({:?})", self.0)
///     }
/// }
/// pub fn lt<T: PartialOrd + std::fmt::Debug>(than: T) -> LtMatchArg<T> {
///     LtMatchArg(than)
/// }
/// ```
macro_rules! simple_matcher {
    (@impl $func_name:ident, $func_name_str:expr, $func_name_link:expr,
     $class_name:ident,
     $comp:tt, $comp_str:expr,
     $msg:expr, $($bounds:tt)+) => {
        to_items! {
            /// This struct is created by the [`
            #[doc = $func_name_str]
            /// `] function. See its documentation for more.
            ///
            /// [`
            #[doc = $func_name_str]
            /// `]:
            #[doc = $func_name_link]
            pub struct $class_name<T>(T);
            impl<T: $($bounds)+ + Debug> MatchArg<T> for $class_name<T> {
                fn matches(&self, arg: &T) -> Result<(), String> {
                    if *arg $comp self.0 {
                        Ok(())
                    } else {
                        Err(format!(concat!("{:?} is ", $msg, " {:?}"), arg, self.0))
                    }
                }

                fn describe(&self) -> String {
                    format!("lt({:?})", self.0)
                }
            }

            /// Matches `value` if 
            #[doc = $comp_str]
            pub fn $func_name<T: $($bounds)+ + Debug>(other: T) -> $class_name<T> {
                $class_name(other)
            }
        }
    };

    ($func_name:ident, $class_name:ident, $comp:tt, $msg:expr, $($bounds:tt)+) => {
        simple_matcher!(
            @impl $func_name, stringify!($func_name),
            // The link needs to be concatenated here rather than in `@impl` to work.
            concat!("fn.", stringify!($func_name), ".html"),
            $class_name,
            // The `value <op> other`. needs to be concatenated here rather than in `@impl`,
            // otherwise, the spacing is funky.
            $comp, concat!("`value ", stringify!($comp), " other`."),
            $msg, $($bounds)+
        );
    };
}

simple_matcher!(lt, LtMatchArg,  <, "not less than", PartialOrd);
simple_matcher!(le, LeMatchArg, <=, "not less than or equal to", PartialOrd);
simple_matcher!(eq, EqMatchArg, ==, "not equal to", PartialEq);
simple_matcher!(ne, NeMatchArg, !=, "equal to", PartialEq);
simple_matcher!(ge, GeMatchArg, >=, "not greater than or equal to", PartialOrd);
simple_matcher!(gt, GtMatchArg,  >, "not greater than", PartialOrd);

/// This struct is created by the [`in_range`] function. See its documentation for more.
///
/// [`in_range`]: fn.in_range.html
pub struct RangeMatchArg<T: Ord + Debug, R: RangeBounds<T>> {
    range: R,
    _phantom: PhantomData<T>,
}
impl<T: Ord + Debug, R: RangeBounds<T>> RangeMatchArg<T, R> {
    fn format_range(&self) -> Result<String, std::fmt::Error> {
        let mut range_str = String::new();
        match self.range.start_bound() {
            Bound::Included(s) => write!(range_str, "[{:?}", s)?,
            Bound::Excluded(s) => write!(range_str, "({:?}", s)?,
            Bound::Unbounded => {}
        }
        range_str.write_char(';')?;
        match self.range.end_bound() {
            Bound::Included(e) => write!(range_str, "{:?}]", e)?,
            Bound::Excluded(e) => write!(range_str, "{:?})", e)?,
            Bound::Unbounded => {}
        }
        Ok(range_str)
    }
}
impl<T: Ord + Debug, R: RangeBounds<T>> MatchArg<T> for RangeMatchArg<T, R> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        let matches_start = match self.range.start_bound() {
            Bound::Included(s) => arg >= s,
            Bound::Excluded(s) => arg > s,
            Bound::Unbounded => true,
        };
        let matches_end = match self.range.end_bound() {
            Bound::Included(s) => arg <= s,
            Bound::Excluded(s) => arg < s,
            Bound::Unbounded => true,
        };
        if matches_start && matches_end {
            Ok(())
        } else {
            Err(format!(
                "{:?} is not in range {}",
                arg,
                self.format_range().unwrap()
            ))
        }
    }

    fn describe(&self) -> String {
        format!("in_range({})", self.format_range().unwrap())
    }
}

/// Matches an argument in a range.
///
/// # Example
/// ```rust
/// # use mockers::MatchArg;
/// # use mockers::matchers::in_range;
///
/// assert!(in_range(0..100).matches(&4).is_ok());
/// ```
pub fn in_range<T: Ord + Debug, R: RangeBounds<T>>(range: R) -> RangeMatchArg<T, R> {
    RangeMatchArg {
        range: range,
        _phantom: PhantomData,
    }
}

/// This struct is created by the [`not`] function. See its documentation for more.
///
/// [`not`]: fn.not.html
pub struct NotMatchArg<T: Debug, M: MatchArg<T>>(M, PhantomData<T>);
impl<T: Debug, M: MatchArg<T>> MatchArg<T> for NotMatchArg<T, M> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        match self.0.matches(arg) {
            Err(_) => Ok(()),
            Ok(()) => Err(format!(
                "{:?} matches (but shouldn't): {}",
                arg,
                self.0.describe()
            )),
        }
    }

    fn describe(&self) -> String {
        format!("lt({:?})", self.0.describe())
    }
}

/// Matches an argument that does not match another matcher.
///
/// # Example
/// ```rust
/// # use mockers::MatchArg;
/// # use mockers::matchers::in_range;
/// # use mockers::matchers::not;
///
/// assert!(not(in_range(0..100)).matches(&400).is_ok());
/// ```
pub fn not<T: Debug, M: MatchArg<T>>(matcher: M) -> NotMatchArg<T, M> {
    NotMatchArg(matcher, PhantomData)
}

/// This struct is created by the [`and`] function. See its documentation for more.
///
/// [`and`]: fn.and.html
pub struct AndMatchArg<T: Debug, M0: MatchArg<T>, M1: MatchArg<T>>(M0, M1, PhantomData<T>);
impl<T: Debug, M0: MatchArg<T>, M1: MatchArg<T>> MatchArg<T> for AndMatchArg<T, M0, M1> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        match self.0.matches(arg) {
            err @ Err(_) => err,
            Ok(()) => match self.1.matches(arg) {
                err @ Err(_) => err,
                Ok(()) => Ok(()),
            },
        }
    }

    fn describe(&self) -> String {
        format!("and({}, {})", self.0.describe(), self.1.describe())
    }
}

/// Matches an argument that matches both argument matcher parameters.
///
/// # Example
/// ```rust
/// # use mockers::MatchArg;
/// # use mockers::matchers::and;
/// # use mockers::matchers::check;
/// # use mockers::matchers::in_range;
///
/// assert!(and(in_range(0..100), check(|i| i % 2 == 0)).matches(&4).is_ok());
/// ```
pub fn and<T: Debug, M0: MatchArg<T>, M1: MatchArg<T>>(
    matcher0: M0,
    matcher1: M1,
) -> AndMatchArg<T, M0, M1> {
    AndMatchArg(matcher0, matcher1, PhantomData)
}

/// This struct is created by the [`or`] function. See its documentation for more.
///
/// [`or`]: fn.or.html
pub struct OrMatchArg<T: Debug, M0: MatchArg<T>, M1: MatchArg<T>>(M0, M1, PhantomData<T>);
impl<T: Debug, M0: MatchArg<T>, M1: MatchArg<T>> MatchArg<T> for OrMatchArg<T, M0, M1> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        match self.0.matches(arg) {
            Ok(()) => Ok(()),
            Err(err0) => match self.1.matches(arg) {
                Ok(()) => Ok(()),
                Err(err1) => Err(format!("{} neither {}", err0, err1)),
            },
        }
    }

    fn describe(&self) -> String {
        format!("or({}, {})", self.0.describe(), self.1.describe())
    }
}

/// Matches an argument that matches either argument matcher parameters.
///
/// # Example
/// ```rust
/// # use mockers::MatchArg;
/// # use mockers::matchers::or;
///
/// assert!(or(4, 42).matches(&4).is_ok());
/// ```
pub fn or<T: Debug, M0: MatchArg<T>, M1: MatchArg<T>>(
    matcher0: M0,
    matcher1: M1,
) -> OrMatchArg<T, M0, M1> {
    OrMatchArg(matcher0, matcher1, PhantomData)
}

pub struct FnMatchArg<T, F: Fn(&T) -> Result<(), String>> {
    func: F,
    _phantom: PhantomData<T>,
}
impl<T, F: Fn(&T) -> Result<(), String>> FnMatchArg<T, F> {
    pub fn new(func: F) -> Self {
        FnMatchArg {
            func: func,
            _phantom: PhantomData,
        }
    }
}
impl<T, F: Fn(&T) -> Result<(), String>> MatchArg<T> for FnMatchArg<T, F> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        let func = &self.func;
        func(arg)
    }
    fn describe(&self) -> String {
        "<function>".to_owned()
    }
}

/// This struct is created by the [`check`] function. See its documentation for more.
///
/// [`check`]: fn.check.html
pub struct BoolFnMatchArg<T, F: Fn(&T) -> bool> {
    func: F,
    _phantom: PhantomData<T>,
}
impl<T, F: Fn(&T) -> bool> BoolFnMatchArg<T, F> {
    pub fn new(func: F) -> Self {
        BoolFnMatchArg {
            func: func,
            _phantom: PhantomData,
        }
    }
}
impl<T, F: Fn(&T) -> bool> MatchArg<T> for BoolFnMatchArg<T, F> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        let func = &self.func;
        if func(arg) {
            Ok(())
        } else {
            Err("<custom function>".to_owned())
        }
    }
    fn describe(&self) -> String {
        "<custom function>".to_owned()
    }
}

/// Matches an argument that satisfies a predicate.
/// See also [`check!`].
///
/// [`check!`]: ../macro.check.html
pub fn check<T, F: Fn(&T) -> bool>(f: F) -> BoolFnMatchArg<T, F> {
    BoolFnMatchArg {
        func: f,
        _phantom: PhantomData,
    }
}

#[macro_export]
macro_rules! arg {
    ($p:pat) => {{
        use $crate::matchers::MatchArgExt;

        let pattern_str = stringify!($p);
        $crate::matchers::FnMatchArg::new(move |arg| match arg {
            &$p => Ok(()),
            _ => Err(format!("{:?} isn't matched by {}", arg, pattern_str)),
        })
        .with_description_fn(move || format!("arg!({})", pattern_str))
    }};
}

#[macro_export]
macro_rules! check {
    ($e:expr) => {{
        use $crate::matchers::MatchArgExt;

        let lambda_str = stringify!($e);
        $crate::matchers::BoolFnMatchArg::new($e)
            .with_custom_msg(move |arg| format!("{:?} doesn't satisfy to {}", arg, lambda_str))
            .with_description_fn(move || format!("check!({})", lambda_str))
    }};
}
