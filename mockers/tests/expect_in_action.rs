use mockers::Scenario;
///! Tests that expectations may be set from inside expectation action.
use mockers_derive::mocked;

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
    let (a, a_handle) = scenario.create_mock_for::<A<Item = BMock>>();
    scenario.expect(a_handle.create_call().and_call({
        let scenario = scenario.handle();
        move || {
            let (b, b_handle) = scenario.create_mock_for::<B>();
            scenario.expect(b_handle.foo_call().and_return(()));
            b
        }
    }));

    let b = a.create();
    b.foo()
}

/// Tests that expectations established from actions are verified.
#[test]
#[should_panic(expected = "not satisfied:\n`B#0.foo()`")]
fn test_expectation_from_action_are_verified() {
    let scenario = Scenario::new();
    let (a, a_handle) = scenario.create_mock_for::<A<Item = BMock>>();
    scenario.expect(a_handle.create_call().and_call({
        let scenario = scenario.handle();
        move || {
            let (b, b_handle) = scenario.create_mock_for::<B>();
            scenario.expect(b_handle.foo_call().and_return(()));
            b
        }
    }));

    let _ = a.create();
}
