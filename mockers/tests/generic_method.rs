#![feature(specialization)]

///! Test that mockers can mock generic methods.
use mockers_derive::{mocked, register_types};

use mockers::matchers::{any, ANY};
use mockers::Scenario;

register_types!(u32, &str, &u32);

#[mocked]
pub trait A {
    fn foo<T>(&self, a: T);
    fn bar<'a>(&self, a: &'a u32);
    fn baz<'a, T>(&self, a: &'a T);
    fn qux<T: ToString>(&self, a: T);
    fn ret<T>(&self) -> T;
}

#[test]
fn test_generic_method_with_type_param() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.foo_call(any::<u32>()).and_return_default().times(1));
    mock.foo(3u32);
}

#[test]
#[ignore] // Support for references is to be done
fn test_generic_method_with_lifetime() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.bar_call(ANY).and_return_default().times(1));
    mock.bar(&3);
}

#[test]
#[ignore] // Support for references is to be done
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

    scenario.expect(mock.qux_call(any::<u32>()).and_return_default().times(1));
    mock.qux(3u32);
}

#[test]
fn test_generic_method_with_parametrized_return_type() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.ret_call().and_return(2u32));
    assert_eq!(mock.ret::<u32>(), 2);
}

/// Test that usage of unregistered type as parameter of mocked generic method
/// causes descriptive error
#[test]
#[should_panic(expected = "Generic method was called with unknown type parameter")]
fn test_usage_of_unregistered_parameter_type() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.foo_call(1u8).and_return(()));
}

/// Test that when call of generic method with some type parameters is expected
/// call with other type parameters don't match.
#[test]
#[should_panic(expected = "unexpected call to `A#0.foo(2)`")]
fn test_two_instantiations_of_generic_method_dont_match() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();

    scenario.expect(mock.foo_call("foofoo").and_return(()));
    mock.foo::<u32>(2);
}
