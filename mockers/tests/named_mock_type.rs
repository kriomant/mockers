///! Test that mock may be named using attribute parameter.
use mockers_derive::mocked;

use mockers::matchers::ANY;
use mockers::Scenario;

#[mocked(MockForA)]
pub trait A {
    fn foo(&self, a: u32);
}

#[test]
fn test_extern() {
    let scenario = Scenario::new();
    let mock = scenario.create_mock::<MockForA>();
    scenario.expect(mock.foo_call(ANY).and_return(()));
    mock.foo(3);
}
