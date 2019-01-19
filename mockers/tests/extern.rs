///! Test that extern functions may be mocked.
use mockers_derive::mocked;

use mockers::matchers::ANY;
use mockers::Scenario;

#[mocked(Foo)]
extern "Rust" {
    fn foo(arg: u32);
}

#[mocked(Bar)]
extern "Rust" {
    fn bar();
}

#[test]
fn extern_function_can_be_mocked() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<Foo>();

    scenario.expect(mock.foo_call(ANY).and_return_default().times(1));

    unsafe { foo(3) };
}

#[test]
#[should_panic(expected = "Mock Foo for extern block already exists")]
fn only_one_mock_instance_of_same_type_is_allowed() {
    let scenario = Scenario::new();
    let _mock1 = scenario.create_mock::<Foo>();
    let _mock2 = scenario.create_mock::<Foo>();
}

#[test]
fn mocks_of_different_types_can_be_used_simultaneously() {
    let scenario = Scenario::new();
    let foo_mock = scenario.create_mock::<Foo>();
    let bar_mock = scenario.create_mock::<Bar>();

    scenario.expect(foo_mock.foo_call(ANY).and_return_default().times(1));
    scenario.expect(bar_mock.bar_call().and_return_default().times(1));

    unsafe { foo(3) };
    unsafe { bar() };
}
