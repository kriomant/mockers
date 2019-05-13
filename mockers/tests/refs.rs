///! Test that mockers can mock methods with reference parameters.
use mockers_derive::mocked;

use mockers::matchers::{ANY, by_ref};
use mockers::Scenario;

#[mocked]
pub trait A {
    fn foo(&self, a: &u32);
}

#[test]
fn test_any_works_for_refs() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock::<AMock>();

    scenario.expect(handle.foo(ANY).and_return_default().times(1));

    mock.foo(&3);
}

#[test]
fn test_refs_comparison() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock::<AMock>();

    scenario.expect(handle.foo(&2).and_return_default().times(1));

    mock.foo(&2);
}

#[test]
fn test_ref_can_be_matched_by_value() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.foo_call(by_ref(2)).and_return_default().times(1));

    mock.foo(&2);
}
