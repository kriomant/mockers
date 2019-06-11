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

/// Matches argument with value of same type using equality.
impl<T: Eq + std::fmt::Debug> MatchArg<T> for T {
    fn matches(&self, arg: &T) -> Result<(), String> {
        if self == arg {
            Ok(())
        } else {
            Err(format!("{:?} is not equal to {:?}", arg, self))
        }
    }

    fn describe(&self) -> String {
        format!("{:?}", self)
    }
}

pub struct MatchRef<T, M: MatchArg<T>> { inner: M, _phantom_data: PhantomData<T> }
impl<T, M: MatchArg<T>> MatchRef<T, M> {
    pub fn new(inner: M) -> Self { MatchRef { inner, _phantom_data: PhantomData } }
}
impl<T, M: MatchArg<T>> MatchArg<&T> for MatchRef<T, M> {
    fn matches(&self, arg: &&T) -> Result<(), String> {
        self.inner.matches(*arg)
    }

    fn describe(&self) -> String {
        self.inner.describe()
    }
}

pub fn by_ref<T, M: MatchArg<T>>(inner: M) -> MatchRef<T, M> {
    MatchRef::new(inner)
}

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
/// Matches any value.
pub const ANY: MatchAny = MatchAny;

pub struct MatchAnyT<T>(PhantomData<T>);
impl<T> MatchArg<T> for MatchAnyT<T> {
    fn matches(&self, _: &T) -> Result<(), String> {
        Ok(())
    }

    fn describe(&self) -> String {
        "_".to_owned()
    }
}

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
    ($func_name:ident, $class_name:ident, $comp:tt, $msg:expr, $($bounds:tt)+) => {
        to_items! {
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
            pub fn $func_name<T: $($bounds)+ + Debug>(than: T) -> $class_name<T> {
                $class_name(than)
            }
        }
    }
}

simple_matcher!(lt, LtMatchArg,  <, "not less than", PartialOrd);
simple_matcher!(le, LeMatchArg, <=, "not less than or equal to", PartialOrd);
simple_matcher!(eq, EqMatchArg, ==, "not equal to", PartialEq);
simple_matcher!(ne, NeMatchArg, !=, "equal to", PartialEq);
simple_matcher!(ge, GeMatchArg, >=, "not greater than or equal to", PartialOrd);
simple_matcher!(gt, GtMatchArg,  >, "not greater than", PartialOrd);

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

pub fn in_range<T: Ord + Debug, R: RangeBounds<T>>(range: R) -> RangeMatchArg<T, R> {
    RangeMatchArg { range, _phantom: PhantomData }
}

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
pub fn not<T: Debug, M: MatchArg<T>>(matcher: M) -> NotMatchArg<T, M> {
    NotMatchArg(matcher, PhantomData)
}

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
pub fn and<T: Debug, M0: MatchArg<T>, M1: MatchArg<T>>(
    matcher0: M0,
    matcher1: M1,
) -> AndMatchArg<T, M0, M1> {
    AndMatchArg(matcher0, matcher1, PhantomData)
}

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
        FnMatchArg { func, _phantom: PhantomData }
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

pub struct BoolFnMatchArg<T, F: Fn(&T) -> bool> {
    func: F,
    _phantom: PhantomData<T>,
}
impl<T, F: Fn(&T) -> bool> BoolFnMatchArg<T, F> {
    pub fn new(func: F) -> Self {
        BoolFnMatchArg { func, _phantom: PhantomData }
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
