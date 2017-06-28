#![feature(plugin, custom_derive)]
#![plugin(mockers_macros)]

///! Test deriving

extern crate mockers;

use mockers::Scenario;

#[derive(Mock)]
pub trait A {
    fn foo(&self, key: i16, value: i32);
}

#[test]
fn test() {
    let scenario = Scenario::new();
    let _mock = scenario.create_mock_for::<A>();
}
