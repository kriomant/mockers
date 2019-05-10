use mockers::{matchers::ANY, Scenario};
///! Test mocking methods with 'dyn Trait' parameters.
use mockers_derive::mocked;

use std::fmt::Debug;

#[mocked]
pub trait A {
    fn foo(&self, value: &dyn Debug);
}

#[test]
fn test() {
    let scenario = Scenario::new();
    let (mock, _) = scenario.create_mock_for::<A>();

    scenario.expect(mock.foo_call(ANY).and_return(()));

    mock.foo(&32);
}
