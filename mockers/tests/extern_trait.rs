///! Test that `mocked` attribute can be used to mock traits defined in
///! some other module or even crate.

use mockers_derive::mocked;
use mockers::Scenario;

/// Extern trait definition.
/// It is important for test trait not to be part of prelude in order
/// to check module name handling.
#[mocked(HasherMock, extern, module="::std::hash")]
pub trait Hasher {
    fn finish(&self) -> u64;
    fn write(&mut self, bytes: &[u8]);
}

#[test]
fn test_extern_trait() {
    use std::hash::Hasher;

    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock::<HasherMock>();

    scenario.expect(handle.finish().and_return(22));
    assert_eq!(22, mock.finish());
}
