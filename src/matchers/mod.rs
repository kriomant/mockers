
use super::MatchArg;

use std::marker::PhantomData;
use std::fmt::Debug;

pub use self::ext::*;

mod ext;


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

    fn describe(&self) -> String { "_".to_owned() }
}
/// Matches any value.
pub const ANY: MatchAny = MatchAny;


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

pub struct NotMatchArg<T: Debug, M: MatchArg<T>>(M, PhantomData<T>);
impl<T: Debug, M: MatchArg<T>> MatchArg<T> for NotMatchArg<T, M> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        match self.0.matches(arg) {
            Err(_) => Ok(()),
            Ok(()) => Err(format!("{:?} matches (but shouldn't): {}", arg, self.0.describe())),
        }
    }

    fn describe(&self) -> String {
        format!("lt({:?})", self.0.describe())
    }
}
pub fn not<T: Debug, M: MatchArg<T>>(matcher: M) -> NotMatchArg<T, M> {
    NotMatchArg(matcher, PhantomData)
}


pub struct AndMatchArg<T: Debug,
                       M0: MatchArg<T>,
                       M1: MatchArg<T>>(M0, M1, PhantomData<T>);
impl<T: Debug, M0: MatchArg<T>, M1: MatchArg<T>> MatchArg<T> for AndMatchArg<T, M0, M1> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        match self.0.matches(arg) {
            err @ Err(_) => err,
            Ok(()) => match self.1.matches(arg) {
                err @ Err(_) => err,
                Ok(()) => Ok(()),
            }
        }
    }

    fn describe(&self) -> String {
        format!("and({}, {})", self.0.describe(), self.1.describe())
    }
}
pub fn and<T: Debug, M0: MatchArg<T>, M1: MatchArg<T>>(matcher0: M0, matcher1: M1) -> AndMatchArg<T, M0, M1> {
    AndMatchArg(matcher0, matcher1, PhantomData)
}


pub struct OrMatchArg<T: Debug,
                      M0: MatchArg<T>,
                      M1: MatchArg<T>>(M0, M1, PhantomData<T>);
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
pub fn or<T: Debug, M0: MatchArg<T>, M1: MatchArg<T>>(matcher0: M0, matcher1: M1) -> OrMatchArg<T, M0, M1> {
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
pub fn check<T, F: Fn(&T) -> bool>(f: F) -> BoolFnMatchArg<T, F> {
    BoolFnMatchArg { func: f, _phantom: PhantomData }
}
