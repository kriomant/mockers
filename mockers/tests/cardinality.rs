use mockers_derive::mocked;

use mockers::cardinality::never;
use mockers::Scenario;

#[mocked]
pub trait A {
    fn foo(&self);
}

#[test]
fn test_times_exactly_satisfied() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2));

    mock.foo();
    mock.foo();
}

#[test]
#[should_panic(expected = "Some expectations are not satisfied:
`A#0.foo() must be called exactly 2 times, called 1 times`
")]
fn test_times_exactly_not_satisfied_less() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2));

    mock.foo();
}

#[test]
#[should_panic(
    expected = "A#0.foo is called for the 3rd time, but expected to be called exactly 2 times"
)]
fn test_times_exactly_not_satisfied_more() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2));

    mock.foo();
    mock.foo();
    mock.foo();
}

#[test]
fn test_times_range_lower_bound() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2..4));

    mock.foo();
    mock.foo();
}

#[test]
fn test_times_range_upper_bound() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2..4));

    mock.foo();
    mock.foo();
    mock.foo();
}

#[test]
#[should_panic(expected = "A#0.foo() must be called from 2 and less than 4 times, called 1 times")]
fn test_times_range_less() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2..4));

    mock.foo();
}

#[test]
#[should_panic(
    expected = "A#0.foo is called for the 4th time, but expected to be called at most 3 times"
)]
fn test_times_range_more() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2..4));

    mock.foo();
    mock.foo();
    mock.foo();
    mock.foo();
}

#[test]
fn test_times_range_inclusive_lower_bound() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2..=4));

    mock.foo();
    mock.foo();
}

#[test]
fn test_times_range_inclusive_upper_bound() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2..=3));

    mock.foo();
    mock.foo();
    mock.foo();
}

#[test]
#[should_panic(expected = "A#0.foo() must be called from 2 to 4 times, called 1 times")]
fn test_times_range_inclusive_less() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2..=4));

    mock.foo();
}

#[test]
#[should_panic(
    expected = "A#0.foo is called for the 5th time, but expected to be called at most 4 times"
)]
fn test_times_range_inclusive_more() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2..=4));

    mock.foo();
    mock.foo();
    mock.foo();
    mock.foo();
    mock.foo();
}

#[test]
fn test_times_range_from_bound() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2..));

    mock.foo();
    mock.foo();
}

#[test]
fn test_times_range_from_more() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2..));

    mock.foo();
    mock.foo();
    mock.foo();
}

#[test]
#[should_panic(expected = "A#0.foo() must be called from 2 and less than 4 times, called 1 times")]
fn test_times_range_from_less() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(2..4));

    mock.foo();
}

#[test]
fn test_times_range_to_bound() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(..3));

    mock.foo();
    mock.foo();
}

#[test]
fn test_times_range_to_less() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(..3));

    mock.foo();
}

#[test]
#[should_panic(
    expected = "A#0.foo is called for the 3rd time, but expected to be called less than 3 times"
)]
fn test_times_range_to_more() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(..3));

    mock.foo();
    mock.foo();
    mock.foo();
}

#[test]
fn test_times_range_to_inclusive_bound() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(..=2));

    mock.foo();
    mock.foo();
}

#[test]
fn test_times_range_to_inclusive_less() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(..=2));

    mock.foo();
}

#[test]
#[should_panic(
    expected = "A#0.foo is called for the 3rd time, but expected to be called at most 2 times"
)]
fn test_times_range_to_inclusive_more() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(..=2));

    mock.foo();
    mock.foo();
    mock.foo();
}

#[test]
fn test_times_range_full_no_calls() {
    let scenario = Scenario::new();
    let (_mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(..));
}

#[test]
fn test_times_range_full_some_calls() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(..));

    mock.foo();
    mock.foo();
}

#[test]
fn test_times_never_no_call() {
    let scenario = Scenario::new();
    let (_mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(never()));
}

#[test]
#[should_panic(expected = "A#0.foo is called for the 1st time, but expected to be never called")]
fn test_times_never_call() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_default().times(never()));

    mock.foo();
}
