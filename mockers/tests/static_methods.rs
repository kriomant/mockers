///! Test that static methods may be mocked.

use mockers_derive::mocked;

use mockers::Scenario;
use mockers::matchers::ANY;

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
    let mock = scenario.create_mock::<FooMock>();
    let mock_static = scenario.create_mock::<FooMockStatic>();

    scenario.expect(mock.foo_call(ANY).and_return_default().times(1));
    scenario.expect(mock_static.bar_call(ANY).and_return(()));
    scenario.expect(mock_static.baz_call().and_return(()));

    use_foo(mock);
}

#[test]
#[should_panic(expected="Mock FooMockStatic for static methods already exists")]
fn only_one_mock_instance_of_same_type_is_allowed() {
    let scenario = Scenario::new();
    let _mock1 = scenario.create_mock::<FooMockStatic>();
    let _mock2 = scenario.create_mock::<FooMockStatic>();
}

#[test]
fn mocks_of_different_types_can_be_used_simultaneously() {
    let scenario = Scenario::new();
    let foo_mock = scenario.create_mock::<FooMockStatic>();
    let bar_mock = scenario.create_mock::<BarMockStatic>();

    scenario.expect(foo_mock.bar_call(ANY).and_return_default().times(1));
    scenario.expect(bar_mock.bar_call().and_return_default().times(1));

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
    let static_mock = scenario.create_mock::<WithCtorMockStatic>();

    scenario.expect(static_mock.new_call().and_call({
        let scenario = scenario.handle();
        move || {
            let mock = scenario.create_mock::<WithCtorMock>();
            scenario.expect(mock.foo_call().and_return(()));
            mock
        }
    }));

    create_and_use::<WithCtorMock>();
}
