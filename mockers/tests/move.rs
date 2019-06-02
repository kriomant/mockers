use mockers::Scenario;

///! Test that mock may be moved and still be controlled
use mockers_derive::mocked;

#[mocked]
pub trait A {
    fn foo(&self);
}

struct Wrapper(pub Box<dyn A>);
impl Wrapper {
    fn foo(&self) { self.0.foo() }
}

#[test]
fn test() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<dyn A>();

    let wrapper = Wrapper(Box::new(mock));

    scenario.expect(handle.foo().and_return(()));
    wrapper.foo();
}
