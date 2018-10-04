///! Test deriving

extern crate mockers;
extern crate mockers_derive;

use mockers_derive::mocked;

use mockers::Scenario;

#[mocked]
pub trait A {
    fn foo(&self, key: i16, value: i32);
}

#[test]
fn test() {
    let scenario = Scenario::new();
    let _mock = scenario.create_mock_for::<A>();
}
