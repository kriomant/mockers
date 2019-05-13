use mockers;
use mockers_derive::{mock, mocked};

#[mocked]
pub trait A {
    fn foo(&self);
}

pub trait B {
    fn bar(&self);
}

mock! {
  BMock,
  self,
  trait B {
      fn bar(&self);
  }
}

#[test]
fn test_a() {
    let scenario = mockers::Scenario::new();
    let (a, a_handle) = scenario.create_mock_for::<A>();
    scenario.expect(a_handle.foo().and_return(()));
    a.foo();
}

#[test]
fn test_b() {
    let scenario = mockers::Scenario::new();
    let (b, b_handle) = scenario.create_mock::<BMock>();
    scenario.expect(b_handle.bar().and_return(()));
    b.bar();
}

#[cfg(feature="nightly")]
#[test]
fn test_diagnostics() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/*.rs");
}
