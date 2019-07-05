///! Test mocking methods with `self: Box<Self>`.

use std::rc::Rc;

use mockers::Scenario;
use mockers_derive::mocked;

#[mocked]
pub trait A {
    fn foo_box(self: Box<Self>);
    fn foo_rc(self: Rc<Self>);
}

#[test]
fn test_box() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<dyn A>();
    let mock = Box::new(mock);

    scenario.expect(handle.foo_box().and_return(()));
    mock.foo_box();
}

#[test]
fn test_rc() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<dyn A>();
    let mock = Rc::new(mock);

    scenario.expect(handle.foo_rc().and_return(()));
    mock.foo_rc();
}
