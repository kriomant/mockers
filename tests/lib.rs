#![feature(box_raw)]

extern crate mockers;

use std::rc::Rc;
use std::cell::RefCell;
use mockers::{Scenario, ScenarioInternals, Mock,
              MatchArg, IntoMatchArg,
              CallMatch0, CallMatch1};

trait A {
    fn foo(&self);
    fn bar(&self, arg: u32);
    fn baz(&self) -> u32;
}

struct AMock {
    scenario: Rc<RefCell<ScenarioInternals>>,
    mock_id: usize,
}

impl Mock for AMock {
    fn new(id: usize, scenario_int: Rc<RefCell<ScenarioInternals>>) -> Self {
        AMock {
            scenario: scenario_int,
            mock_id: id,
        }
    }
}

impl AMock {
    fn foo(&self) {
        let args = ();
        let args_ptr: *const u8 = unsafe { std::mem::transmute(&args) };
        let result_ptr: *mut u8 = self.scenario.borrow_mut().call(self.mock_id, 0, args_ptr);
        let result: Box<()> = unsafe { Box::from_raw(result_ptr as *mut ()) };
        *result
    }

    fn foo_call(&self) -> CallMatch0<()> {
        CallMatch0::<()>::new(self.mock_id, 0)
    }

    fn bar(&self, arg: u32) {
        let args = (arg);
        let args_ptr: *const u8 = unsafe { std::mem::transmute(&args) };
        let result_ptr: *mut u8 = self.scenario.borrow_mut().call(self.mock_id, 1, args_ptr);
        let result: Box<()> = unsafe { Box::from_raw(result_ptr as *mut ()) };
        *result
    }

    fn bar_call<Arg0Match>(&self, arg0: Arg0Match) -> CallMatch1<u32, ()>
            where Arg0Match: IntoMatchArg<u32> {
        CallMatch1::new(self.mock_id, 1, arg0.into_match_arg())
    }

    fn baz(&self) -> u32 {
        let args = ();
        let args_ptr: *const u8 = unsafe { std::mem::transmute(&args) };
        let result_ptr: *mut u8 = self.scenario.borrow_mut().call(self.mock_id, 2, args_ptr);
        let result: Box<u32> = unsafe { Box::from_raw(result_ptr as *mut u32) };
        *result
    }

    fn baz_call(&self) -> CallMatch0<u32> {
        CallMatch0::<u32>::new(self.mock_id, 2)
    }
}

#[test]
#[should_panic(expected="Unexpected event")]
fn test_unit() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.foo_call().and_return(()));
    mock.bar(2);
}

#[test]
fn test_return() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.baz_call().and_return(2));
    assert_eq!(2, mock.baz());
}

#[cfg(test)]
fn less_than<T: 'static + PartialOrd + std::fmt::Debug>(limit: T) -> MatchArg<T> {
    Box::new(move |value| {
        if value < &limit {
            Ok(())
        } else {
            Err(format!("{:?} is not less than {:?}", value, limit))
        }
    })
}

#[test]
#[should_panic(expected="4 is not less than 3")]
fn test_arg_match_failure() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.bar_call(less_than(3)).and_return(()));
    mock.bar(4);
}

#[test]
fn test_arg_match_success() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<AMock>();
    scenario.expect(mock.bar_call(less_than(3)).and_return(()));
    mock.bar(2);
}

//mock![A];
