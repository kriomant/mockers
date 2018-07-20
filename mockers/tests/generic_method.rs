#![feature(use_extern_macros)]

///! Test that mockers can mock generic methods.

extern crate mockers;
extern crate mockers_derive;

use mockers_derive::mocked;

use mockers::Scenario;
use mockers::matchers::{ANY, any};

#[mocked]
pub trait A {
    fn foo<T>(&self, a: T);
    fn bar<'a>(&self, a: &'a u32);
    fn baz<'a, T>(&self, a: &'a T);
    fn qux<'a, T: ToString>(&self, a: &'a T);
    fn ret<T>(&self) -> T;
}

#[test]
fn test_generic_method_with_type_param() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.foo_call(any::<u32>()).and_return_default().times(1));
    mock.foo(3);
}

#[test]
fn test_generic_method_with_lifetime() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.bar_call(ANY).and_return_default().times(1));
    mock.bar(&3);
}

#[test]
fn test_generic_method_with_type_param_and_lifetime() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.baz_call(any::<&u32>()).and_return_default().times(1));
    mock.baz(&3);
}

#[test]
fn test_generic_method_with_type_param_bounds() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.qux_call(any::<&u32>()).and_return_default().times(1));
    mock.qux(&3);
}

#[test]
fn test_generic_method_with_parametrized_return_type() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.ret_call().and_return(2u32));
    assert_eq!(mock.ret::<u32>(), 2);
}
