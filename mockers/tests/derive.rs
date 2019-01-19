use mockers::Scenario;
///! Test deriving
use mockers_derive::mocked;

#[mocked]
pub trait A {
    fn foo(&self, key: i16, value: i32);
}

#[test]
fn test() {
    let scenario = Scenario::new();
    let _mock = scenario.create_mock_for::<A>();
}
