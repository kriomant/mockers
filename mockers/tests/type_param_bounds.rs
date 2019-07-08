///! Test mocking generic trait with type parameter bounds

use mockers::Scenario;
use mockers_derive::mocked;

#[mocked(debug)]
pub trait A<T: 'static + std::fmt::Display> {
    fn foo(&self, val: T);
}

#[test]
fn test() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<dyn A<u32>>();

    scenario.expect(handle.foo(2).and_return(()));
    mock.foo(2);
}
