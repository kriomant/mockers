///! Test that generated code doesn't conflict with types defined
///! in user code. The most often example is defining local `Result`
///! and `Error` types.
use mockers_derive::mocked;

use mockers::matchers::ANY;
use mockers::Scenario;

type Result<T> = std::result::Result<T, String>;

#[mocked]
pub trait A {
    fn foo(&self, a: &u32) -> Result<u32>;
}

#[test]
fn test_any_works_for_refs() {
    let scenario = Scenario::new();
    let (mock, _) = scenario.create_mock::<AMock>();

    scenario.expect(mock.foo_call(ANY).and_return(Ok(23)));

    assert_eq!(Ok(23), mock.foo(&3));
}
