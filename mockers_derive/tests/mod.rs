#![feature(use_extern_macros)]

extern crate mockers_derive;
extern crate mockers;
use mockers_derive::{mocked, mock};

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
  let a = scenario.create_mock_for::<A>();
  scenario.expect(a.foo_call().and_return(()));
  a.foo();
}

#[test]
fn test_b() {
  let scenario = mockers::Scenario::new();
  let b = scenario.create_mock::<BMock>();
  scenario.expect(b.bar_call().and_return(()));
  b.bar();
}
