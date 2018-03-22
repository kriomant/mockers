#![feature(proc_macro)]

///! Test deriving

extern crate mockers;
extern crate mockers_derive;

use mockers_derive::derive_mock;

use mockers::Scenario;

#[derive_mock]
pub trait A {
    fn foo(&self, key: i16, value: i32);
}

#[test]
fn test() {
    let scenario = Scenario::new();
    let _mock = scenario.create_mock_for::<A>();
}
