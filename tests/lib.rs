#![feature(plugin)]
#![plugin(mockers_macros)]

#[macro_use(arg)]
extern crate mockers;

use mockers::{Scenario, MatchArg, ANY};

mod nested {
    pub trait A {
        fn foo(&self);
        fn bar(&self, arg: u32);
        fn baz(&self) -> u32;
        fn cmplx(&self, maybe: Option<u32>);
    }
}

mock!{
    AMock,
    nested,
    trait A {
        fn foo(&self);
        fn bar(&self, arg: u32);
        fn baz(&self) -> u32;
        fn cmplx(&self, maybe: Option<u32>);
    }
}

use nested::A;

#[test]
#[should_panic(expected="Unexpected call of `foo`, `bar(2)` call is expected")]
fn test_unit() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.bar_call(2).and_return(()));
    mock.foo();
}

#[test]
fn test_return() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.baz_call().and_return(2));
    assert_eq!(2, mock.baz());
}

#[cfg(test)]
struct LessThanMatchArg<T>(T);
impl<T: PartialOrd + std::fmt::Debug + 'static> MatchArg<T> for LessThanMatchArg<T> {
    fn matches(&self, arg: &T) -> Result<(), String> {
        if arg < &self.0 {
            Ok(())
        } else {
            Err(format!("{:?} is not less than {:?}", arg, self.0))
        }
    }

    fn describe(&self) -> String {
        format!("less_than({:?})", self.0)
    }
}
#[cfg(test)]
fn less_than<T: 'static + PartialOrd + std::fmt::Debug>(limit: T) -> LessThanMatchArg<T> {
    LessThanMatchArg(limit)
}

#[test]
#[should_panic(expected="4 is not less than 3")]
fn test_arg_match_failure() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.bar_call(less_than(3)).and_return(()));
    mock.bar(4);
}

#[test]
fn test_arg_match_success() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.bar_call(less_than(3)).and_return(()));
    mock.bar(2);
}

#[test]
fn test_any_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.bar_call(ANY).and_return(()));
    mock.bar(2);
}

#[test]
#[should_panic(expected="Expected calls are not performed:\n`bar(_)`\n")]
fn test_expected_call_not_performed() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.bar_call(ANY).and_return(()));
}

#[test]
fn test_arg_macro_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.cmplx_call(arg!(Some(_))).and_return(()));
    mock.cmplx(Some(3));
}

#[test]
#[should_panic(expected="None isn\\'t matched by Some(_)")]
fn test_arg_macro_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.cmplx_call(arg!(Some(_))).and_return(()));
    mock.cmplx(None);
}
