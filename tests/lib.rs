#![feature(box_raw)]

use std::marker::PhantomData;
use std::rc::Rc;
use std::cell::RefCell;

trait CheckCall {
    fn check_call(self: Box<Self>, args: *const u8) -> *mut u8;
    fn get_mock_id(&self) -> usize;
    fn get_method_id(&self) -> usize;
}

#[must_use]
struct CallMatch0<Res> {
    mock_id: usize,
    method_id: usize,

    _phantom: PhantomData<Res>,
}
#[must_use]
struct Expectation0<Res> {
    call_match: CallMatch0<Res>,
    result: Res,
}
impl<Res> Expectation0<Res> {
    fn check(self) -> Res { self.result }
}
impl<Res> CheckCall for Expectation0<Res> {
    fn check_call(self: Box<Self>, args: *const u8) -> *mut u8 {
        let args_tuple: &() = unsafe { std::mem::transmute(args) };
        let result = self.check();
        Box::into_raw(Box::new(result)) as *mut u8
    }
    fn get_mock_id(&self) -> usize { self.call_match.mock_id }
    fn get_method_id(&self) -> usize { self.call_match.method_id }
}
impl<Res> CallMatch0<Res> {
    fn and_return(self, result: Res) -> Expectation0<Res> {
        Expectation0 { call_match: self, result: result }
    }
}

#[must_use]
struct CallMatch1<Arg0, Res> {
    mock_id: usize,
    method_id: usize,
    arg0: MatchArg<Arg0>,

    _phantom: PhantomData<Res>,
}
#[must_use]
struct Expectation1<Arg0, Res> {
    call_match: CallMatch1<Arg0, Res>,
    result: Res,
}
impl<Arg0, Res> Expectation1<Arg0, Res> {
    fn check(self, arg0: &Arg0) -> Res {
        (*self.call_match.arg0)(arg0).unwrap();
        self.result
    }
}
impl<Arg0, Res> CheckCall for Expectation1<Arg0, Res> {
    fn check_call(self: Box<Self>, args: *const u8) -> *mut u8 {
        let args_tuple: &(Arg0,) = unsafe { std::mem::transmute(args) };
        let result = self.check(&args_tuple.0);
        Box::into_raw(Box::new(result)) as *mut u8
    }
    fn get_mock_id(&self) -> usize { self.call_match.mock_id }
    fn get_method_id(&self) -> usize { self.call_match.method_id }
}
impl<Arg0, Res> CallMatch1<Arg0, Res> {
    fn and_return(self, result: Res) -> Expectation1<Arg0, Res> {
        Expectation1 { call_match: self, result: result }
    }
}

#[must_use]
struct CallMatch2<Arg0, Arg1, Res> {
    mock_id: usize,
    method_id: usize,
    arg0: MatchArg<Arg0>,
    arg1: MatchArg<Arg1>,

    _phantom: PhantomData<Res>,
}
#[must_use]
struct Expectation2<Arg0, Arg1, Res> {
    call_match: CallMatch2<Arg0, Arg1, Res>,
    result: Res,
}
impl <Arg0, Arg1, Res> Expectation2<Arg0, Arg1, Res> {
    fn check(self, arg0: &Arg0, arg1: &Arg1) -> Res {
        (*self.call_match.arg0)(arg0).unwrap();
        (*self.call_match.arg1)(arg1).unwrap();
        self.result
    }
}
impl<Arg0, Arg1, Res> CheckCall for Expectation2<Arg0, Arg1, Res> {
    fn check_call(self: Box<Self>, args: *const u8) -> *mut u8 {
        let args_tuple: &(Arg0, Arg1) = unsafe { std::mem::transmute(args) };
        let result = self.check(&args_tuple.0, &args_tuple.1);
        Box::into_raw(Box::new(result)) as *mut u8
    }
    fn get_mock_id(&self) -> usize { self.call_match.mock_id }
    fn get_method_id(&self) -> usize { self.call_match.method_id }
}
impl<Arg0, Arg1, Res> CallMatch2<Arg0, Arg1, Res> {
    fn and_return(self, result: Res) -> Expectation2<Arg0, Arg1, Res> {
        Expectation2 { call_match: self, result: result }
    }
}

type MatchArg<T> = Box<Fn(&T) -> Result<(), String>>;

trait IntoMatchArg<T> {
    fn into_match_arg(self) -> MatchArg<T>;
}

impl<'a, T> IntoMatchArg<T> for T
    where T: 'static + Eq + std::fmt::Debug {

    fn into_match_arg(self) -> MatchArg<T> {
        Box::new(move |value| {
            if *value == self {
                Ok(())
            } else {
                Err(format!("{:?} is not equal to {:?}", value, self))
            }
        })
    }
}

trait A {
    fn foo(&self);
    fn bar(&self, arg: u32);
    fn baz(&self) -> u32;
}

struct AMock {
    scenario: Rc<RefCell<ScenarioInternals>>,
    mock_id: usize,
}

impl AMock {
    fn new(id: usize, scenario_int: Rc<RefCell<ScenarioInternals>>) -> Self {
        AMock {
            scenario: scenario_int,
            mock_id: id,
        }
    }

    fn foo(&self) {
        let args = ();
        let args_ptr: *const u8 = unsafe { std::mem::transmute(&args) };
        let result_ptr: *mut u8 = self.scenario.borrow_mut().call(self.mock_id, 0, args_ptr);
        let result: Box<()> = unsafe { Box::from_raw(result_ptr as *mut ()) };
        *result
    }

    fn foo_call(&self) -> CallMatch0<()> {
        CallMatch0 {
            mock_id: self.mock_id,
            method_id: 0,

            _phantom: PhantomData,
        }
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
        CallMatch1 {
            mock_id: self.mock_id,
            method_id: 1,
            arg0: arg0.into_match_arg(),

            _phantom: PhantomData,
        }
    }

    fn baz(&self) -> u32 {
        let args = ();
        let args_ptr: *const u8 = unsafe { std::mem::transmute(&args) };
        let result_ptr: *mut u8 = self.scenario.borrow_mut().call(self.mock_id, 2, args_ptr);
        let result: Box<u32> = unsafe { Box::from_raw(result_ptr as *mut u32) };
        *result
    }

    fn baz_call(&self) -> CallMatch0<u32> {
        CallMatch0 {
            mock_id: self.mock_id,
            method_id: 2,

            _phantom: PhantomData,
        }
    }
}

struct ScenarioInternals {
    events: Vec<Box<CheckCall>>,
}

struct Scenario {
    internals: Rc<RefCell<ScenarioInternals>>,
    next_mock_id: usize,
}

impl Scenario {
    pub fn new() -> Self {
        Scenario {
            internals: Rc::new(RefCell::new(ScenarioInternals {
                events: Vec::new(),
            })),
            next_mock_id: 0,
        }
    }

    pub fn create_mock(&mut self) -> AMock {
        AMock::new(self.get_next_mock_id(), self.internals.clone())
    }

    fn get_next_mock_id(&mut self) -> usize {
        let id = self.next_mock_id;
        self.next_mock_id += 1;
        id
    }

    pub fn expect<C: CheckCall + 'static>(&mut self, call: C) {
        self.internals.borrow_mut().events.push(Box::new(call));
    }
}

impl ScenarioInternals {
    pub fn call(&mut self, mock_id: usize, method_id: usize, args_ptr: *const u8) -> *mut u8 {
        use std::ops::Deref;

        let event = self.events.remove(0);
        if event.get_mock_id() != mock_id || event.get_method_id() != method_id {
            panic!("Unexpected event");
        }
        event.check_call(args_ptr)
    }
}

#[test]
#[should_panic(expected="Unexpected event")]
fn test_unit() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock();
    scenario.expect(mock.foo_call().and_return(()));
    mock.bar(2);
}

#[test]
fn test_return() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock();
    scenario.expect(mock.baz_call().and_return(2));
    assert_eq!(2, mock.baz());
}
