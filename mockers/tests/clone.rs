#![feature(use_extern_macros)]

#[macro_use]
extern crate mockers;
extern crate mockers_derive;

use mockers_derive::mock;
use mockers::Scenario;

pub trait A {
    fn foo(&self, a: u32);
}

// This mock shares expectations between clones.
mock!{
    AShared,
    self,
    trait A {
        fn foo(&self, a: u32);
    }
}
mock_clone!(AShared, share_expectations);

// This mock mocks `clone` method.
mock!{
    AMock,
    self,
    trait A {
        fn foo(&self, a: u32);
    }
}
mock_clone!(AMock);

fn target<AC: A + Clone>(a: AC) {
    let clone = a.clone();
    clone.foo(2);
}

#[test]
fn test_shared() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AShared>();

    scenario.expect(mock.foo_call(2).and_return_default().times(1));

    target(mock);
}

#[test]
fn test_clone_mock() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    let mock_clone = scenario.create_mock::<AMock>();

    scenario.expect(mock_clone.foo_call(2).and_return_default().times(1));
    scenario.expect(mock.clone_call().and_return(mock_clone));

    target(mock);
}

// Test that it is possible to create mock right from `clone` expectation reaction.
#[test]
fn test_clone_mock_dynamic() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.clone_call().and_call({
        let scenario = scenario.handle();
        move || {
            let clone = scenario.create_mock::<AMock>();
            scenario.expect(clone.foo_call(2).and_return_default().times(1));
            clone
        }
    }));

    target(mock);
}
