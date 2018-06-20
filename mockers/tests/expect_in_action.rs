#![feature(proc_macro)]

///! Tests that expectations may be set from inside expectation action.

extern crate mockers;
extern crate mockers_derive;

use mockers_derive::mocked;
use mockers::Scenario;

#[mocked]
pub trait A {
    type Item;
    fn create(&self) -> Self::Item;
}

#[mocked]
pub trait B {
    fn foo(&self);
}

/// Tests that mock may be created for trait with associated types.
#[test]
fn test_expectation_from_action() {
    let scenario = Scenario::new();
    let a = scenario.create_mock_for::<A<Item=BMock>>();
    scenario.expect(a.create_call().and_call({
      let scenario = scenario.handle();
      move || {
        let b = scenario.create_mock_for::<B>();
        scenario.expect(b.foo_call().and_return(()));
        b
      }
    }));

    let b = a.create();
    b.foo()
}

/// Tests that expectations established from actions are verified.
#[test]
#[should_panic(expected="not satisfied:\n`B#0.foo()`")]
fn test_expectation_from_action_are_verified() {
    let scenario = Scenario::new();
    let a = scenario.create_mock_for::<A<Item=BMock>>();
    scenario.expect(a.create_call().and_call({
      let scenario = scenario.handle();
      move || {
        let b = scenario.create_mock_for::<B>();
        scenario.expect(b.foo_call().and_return(()));
        b
      }
    }));

    let _ = a.create();
}
