#![feature(proc_macro)]

extern crate mockers_derive;
extern crate mockers;
use mockers_derive::{derive_mock, mock};

#[derive_mock]
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
