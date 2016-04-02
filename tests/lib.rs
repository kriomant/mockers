#![feature(plugin)]
#![plugin(mockers_macros)]

extern crate mockers;

use std::rc::Rc;
use std::cell::RefCell;
use mockers::{Scenario, ScenarioInternals, Mock,
              MatchArg, IntoMatchArg, ANY};

trait A {
    fn foo(&self);
    fn bar(&self, arg: u32);
    fn baz(&self) -> u32;
}

mock!{
    AMock,
    trait A {
        fn foo(&self);
        fn bar(&self, arg: u32);
        fn baz(&self) -> u32;
    }
}

#[test]
#[should_panic(expected="Unexpected event")]
fn test_unit() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.foo_call().and_return(()));
    mock.bar(2);
}

#[test]
fn test_return() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.baz_call().and_return(2));
    assert_eq!(2, mock.baz());
}

#[cfg(test)]
fn less_than<T: 'static + PartialOrd + std::fmt::Debug>(limit: T) -> MatchArg<T> {
    Box::new(move |value| {
        if value < &limit {
            Ok(())
        } else {
            Err(format!("{:?} is not less than {:?}", value, limit))
        }
    })
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
