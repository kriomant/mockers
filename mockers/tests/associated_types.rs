///! Test that traits with associated types can be mocked.

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
    type Item;
    fn create(&self, item: Self::Item) -> Vec<(bool, Self::Item)>;
}

/// Tests that mock may be created for trait with associated types.
#[test]
fn test_assocated_type() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock_for::<A<Item=i32>>();
    scenario.expect(mock.create_call().and_return(2));
    assert_eq!(mock.create(), 2);
}

/// Tests that all references to `Self` in trait definition are
/// properly qualified with trait path in function signatures.
#[test]
fn test_self_type_qualification() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock_for::<B<Item=i32>>();
    scenario.expect(mock.create_call(1).and_return(vec![(true, 2)]));
    assert_eq!(mock.create(1), vec![(true, 2)]);
}
