#[macro_use]
extern crate mockers;

use mockers::Scenario;
use mockers_derive::mock;

use mockers::CloneHandle as _;

pub trait A {
    fn foo(&self, a: u32);
}

// This mock shares expectations between clones.
mock! {
    AShared,
    self,
    trait A {
        fn foo(&self, a: u32);
    }
}
mock_clone!(AShared, AMockHandle, share_expectations);

// This mock mocks `clone` method.
mock! {
    AMock,
    self,
    trait A {
        fn foo(&self, a: u32);
    }
}
mock_clone!(AMock, AMockHandle);

fn target<AC: A + Clone>(a: AC) {
    let clone = a.clone();
    clone.foo(2);
}

#[test]
fn test_shared() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock::<AShared>();

    scenario.expect(handle.foo(2).and_return_default().times(1));

    target(mock);
}

#[test]
fn test_clone_mock() {
    let scenario = Scenario::new();
    let (mock, mock_handle) = scenario.create_mock::<AMock>();
    let (mock_clone, mock_clone_handle) = scenario.create_mock::<AMock>();

    scenario.expect(mock_clone_handle.foo(2).and_return_default().times(1));
    scenario.expect(mock_handle.clone().and_return(mock_clone));

    target(mock);
}

// Test that it is possible to create mock right from `clone` expectation reaction.
#[test]
fn test_clone_mock_dynamic() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock::<AMock>();

    scenario.expect(handle.clone().and_call({
        let scenario = scenario.handle();
        move || {
            let (clone, clone_handle) = scenario.create_mock::<AMock>();
            scenario.expect(clone_handle.foo(2).and_return_default().times(1));
            clone
        }
    }));

    target(mock);
}
