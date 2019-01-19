///! Test mocking methods with 'dyn Trait' parameters.

extern crate mockers;
extern crate mockers_derive;

use mockers_derive::mocked;
use mockers::{Scenario, matchers::ANY};

use std::fmt::Debug;

#[mocked]
pub trait A {
    fn foo(&self, value: &dyn Debug);
}

#[test]
fn test() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock_for::<A>();

    scenario.expect(mock.foo_call(ANY).and_return(()));

    mock.foo(&32);
}
