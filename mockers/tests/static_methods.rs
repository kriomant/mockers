///! Test that static methods may be mocked.
use mockers_derive::mocked;

use mockers::matchers::ANY;
use mockers::Scenario;

#[mocked]
trait Foo {
    fn foo(&self, arg: u32);
    fn bar(arg: u32);
    fn baz();
}

#[mocked]
trait Bar {
    fn bar();
}

fn use_foo<F: Foo>(f: F) {
    f.foo(3);
    F::bar(2);
    F::baz();
}

#[test]
fn static_methods_can_be_mocked() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock::<FooMock>();
    let (_mock_static, static_handle) = scenario.create_mock::<FooMockStatic>();

    scenario.expect(handle.foo(ANY).and_return_default().times(1));
    scenario.expect(static_handle.bar(ANY).and_return(()));
    scenario.expect(static_handle.baz().and_return(()));

    use_foo(mock);
}

#[test]
#[should_panic(expected = "Mock FooMockStatic for static methods already exists")]
fn only_one_mock_instance_of_same_type_is_allowed() {
    let scenario = Scenario::new();
    let (_mock1, _) = scenario.create_mock::<FooMockStatic>();
    let (_mock2, _) = scenario.create_mock::<FooMockStatic>();
}

#[test]
fn mocks_of_different_types_can_be_used_simultaneously() {
    let scenario = Scenario::new();
    let (_foo_mock, foo_handle) = scenario.create_mock::<FooMockStatic>();
    let (_bar_mock, bar_handle) = scenario.create_mock::<BarMockStatic>();

    scenario.expect(foo_handle.bar(ANY).and_return_default().times(1));
    scenario.expect(bar_handle.bar().and_return_default().times(1));

    FooMock::bar(3);
    BarMock::bar();
}

#[mocked]
trait WithCtor {
    fn new() -> Self;
    fn foo(&self);
}

fn create_and_use<T: WithCtor>() {
    let t = T::new();
    t.foo();
}

#[test]
fn mock_trait_with_ctor() {
    let scenario = Scenario::new();
    let (_static_mock, static_handle) = scenario.create_mock::<WithCtorMockStatic>();

    scenario.expect(static_handle.new().and_call({
        let scenario = scenario.handle();
        move || {
            let (mock, handle) = scenario.create_mock::<WithCtorMock>();
            scenario.expect(handle.foo().and_return(()));
            mock
        }
    }));

    create_and_use::<WithCtorMock>();
}
