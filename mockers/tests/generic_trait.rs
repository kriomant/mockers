///! Test that generic traits can be mocked.

use mockers::Scenario;
use mockers_derive::mocked;

#[mocked]
pub trait A<T> {
    fn put(&self, t: T);
}

#[test]
fn test_generic_trait() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A<u32>>();
    scenario.expect(handle.put(2).and_return(()));
    mock.put(2);
}

#[mocked]
pub trait B<T> {
    type Item;
    fn put(&self, t: T) -> Self::Item;
}

#[test]
fn test_generic_trait_with_type_member() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<B<u32, Item=u64>>();
    scenario.expect(handle.put(2).and_return(4));
    assert_eq!(4, mock.put(2));
}
