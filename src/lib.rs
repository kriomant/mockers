use std::marker::PhantomData;
use std::rc::Rc;
use std::cell::RefCell;

pub mod matchers;

enum MockCallResult<T> {
    Return(T),
    Panic(String),
}
impl<T> MockCallResult<T> {
    fn get(self) -> T {
        match self {
            MockCallResult::Return(value) => value,
            MockCallResult::Panic(msg) => panic!(msg),
        }
    }
}

pub trait CheckCall {
    fn check_call(self: Box<Self>, args: *const u8) -> *mut u8;
    fn get_mock_id(&self) -> usize;
    fn get_method_name(&self) -> &'static str;
    fn describe(&self) -> String;
}

#[must_use]
pub struct CallMatch0<Res> {
    mock_id: usize,
    method_name: &'static str,

    _phantom: PhantomData<Res>,
}
impl<Res> CallMatch0<Res> {
    pub fn new(mock_id: usize, method_name: &'static str) -> Self {
        CallMatch0 {
            mock_id: mock_id,
            method_name: method_name,
            _phantom: PhantomData
        }
    }
}

#[must_use]
pub struct Expectation0<Res> {
    call_match: CallMatch0<Res>,
    result: MockCallResult<Res>,
}
impl<Res> Expectation0<Res> {
    fn check(self) -> Res { self.result.get() }
}
impl<Res> CheckCall for Expectation0<Res> {
    fn check_call(self: Box<Self>, _args: *const u8) -> *mut u8 {
        //let args_tuple: &() = unsafe { std::mem::transmute(args) };
        let result = self.check();
        Box::into_raw(Box::new(result)) as *mut u8
    }
    fn get_mock_id(&self) -> usize { self.call_match.mock_id }
    fn get_method_name(&self) -> &'static str { self.call_match.method_name }
    fn describe(&self) -> String {
        format!("{}()", self.get_method_name())
    }
}
impl<Res> CallMatch0<Res> {
    pub fn and_return(self, result: Res) -> Expectation0<Res> {
        Expectation0 { call_match: self, result: MockCallResult::Return(result) }
    }

    pub fn and_panic(self, msg: String) -> Expectation0<Res> {
        Expectation0 { call_match: self, result: MockCallResult::Panic(msg) }
    }
}

#[must_use]
pub struct CallMatch1<Arg0, Res> {
    mock_id: usize,
    method_name: &'static str,
    arg0: Box<MatchArg<Arg0>>,

    _phantom: PhantomData<Res>,
}
impl<Arg0, Res> CallMatch1<Arg0, Res> {
    pub fn new(mock_id: usize, method_name: &'static str, arg0: Box<MatchArg<Arg0>>) -> Self {
        CallMatch1 {
            mock_id: mock_id,
            method_name: method_name,
            arg0: arg0,
            _phantom: PhantomData
        }
    }
}

#[must_use]
pub struct Expectation1<Arg0, Res> {
    call_match: CallMatch1<Arg0, Res>,
    result: MockCallResult<Res>,
}
impl<Arg0, Res> Expectation1<Arg0, Res> {
    fn check(self, arg0: &Arg0) -> Res {
        self.call_match.arg0.matches(arg0).unwrap();
        self.result.get()
    }
}
impl<Arg0, Res> CheckCall for Expectation1<Arg0, Res> {
    fn check_call(self: Box<Self>, args: *const u8) -> *mut u8 {
        let args_tuple: &(Arg0,) = unsafe { std::mem::transmute(args) };
        let result = self.check(&args_tuple.0);
        Box::into_raw(Box::new(result)) as *mut u8
    }
    fn get_mock_id(&self) -> usize { self.call_match.mock_id }
    fn get_method_name(&self) -> &'static str { self.call_match.method_name }
    fn describe(&self) -> String {
        format!("{}({})", self.get_method_name(),
                          self.call_match.arg0.describe())
    }
}
impl<Arg0, Res> CallMatch1<Arg0, Res> {
    pub fn and_return(self, result: Res) -> Expectation1<Arg0, Res> {
        Expectation1 { call_match: self, result: MockCallResult::Return(result) }
    }

    pub fn and_panic(self, msg: String) -> Expectation1<Arg0, Res> {
        Expectation1 { call_match: self, result: MockCallResult::Panic(msg) }
    }
}

#[must_use]
pub struct CallMatch2<Arg0, Arg1, Res> {
    mock_id: usize,
    method_name: &'static str,
    arg0: Box<MatchArg<Arg0>>,
    arg1: Box<MatchArg<Arg1>>,

    _phantom: PhantomData<Res>,
}
impl<Arg0, Arg1, Res> CallMatch2<Arg0, Arg1, Res> {
    pub fn new(mock_id: usize, method_name: &'static str,
               arg0: Box<MatchArg<Arg0>>,
               arg1: Box<MatchArg<Arg1>>) -> Self {
        CallMatch2 {
            mock_id: mock_id,
            method_name: method_name,
            arg0: arg0,
            arg1: arg1,
            _phantom: PhantomData
        }
    }
}

#[must_use]
pub struct Expectation2<Arg0, Arg1, Res> {
    call_match: CallMatch2<Arg0, Arg1, Res>,
    result: MockCallResult<Res>,
}
impl <Arg0, Arg1, Res> Expectation2<Arg0, Arg1, Res> {
    fn check(self, arg0: &Arg0, arg1: &Arg1) -> Res {
        self.call_match.arg0.matches(arg0).unwrap();
        self.call_match.arg1.matches(arg1).unwrap();
        self.result.get()
    }
}
impl<Arg0, Arg1, Res> CheckCall for Expectation2<Arg0, Arg1, Res> {
    fn check_call(self: Box<Self>, args: *const u8) -> *mut u8 {
        let args_tuple: &(Arg0, Arg1) = unsafe { std::mem::transmute(args) };
        let result = self.check(&args_tuple.0, &args_tuple.1);
        Box::into_raw(Box::new(result)) as *mut u8
    }
    fn get_mock_id(&self) -> usize { self.call_match.mock_id }
    fn get_method_name(&self) -> &'static str { self.call_match.method_name }
    fn describe(&self) -> String {
        format!("{}({}, {})", self.get_method_name(),
                              self.call_match.arg0.describe(),
                              self.call_match.arg1.describe())
    }
}
impl<Arg0, Arg1, Res> CallMatch2<Arg0, Arg1, Res> {
    pub fn and_return(self, result: Res) -> Expectation2<Arg0, Arg1, Res> {
        Expectation2 { call_match: self, result: MockCallResult::Return(result) }
    }

    pub fn and_panic(self, msg: String) -> Expectation2<Arg0, Arg1, Res> {
        Expectation2 { call_match: self, result: MockCallResult::Panic(msg) }
    }
}


#[must_use]
pub struct CallMatch3<Arg0, Arg1, Arg2, Res> {
    mock_id: usize,
    method_name: &'static str,
    arg0: Box<MatchArg<Arg0>>,
    arg1: Box<MatchArg<Arg1>>,
    arg2: Box<MatchArg<Arg2>>,

    _phantom: PhantomData<Res>,
}
impl<Arg0, Arg1, Arg2, Res> CallMatch3<Arg0, Arg1, Arg2, Res> {
    pub fn new(mock_id: usize, method_name: &'static str,
               arg0: Box<MatchArg<Arg0>>,
               arg1: Box<MatchArg<Arg1>>,
               arg2: Box<MatchArg<Arg2>>) -> Self {
        CallMatch3 {
            mock_id: mock_id,
            method_name: method_name,
            arg0: arg0,
            arg1: arg1,
            arg2: arg2,
            _phantom: PhantomData
        }
    }
}


#[must_use]
pub struct Expectation3<Arg0, Arg1, Arg2, Res> {
    call_match: CallMatch3<Arg0, Arg1, Arg2, Res>,
    result: MockCallResult<Res>,
}
impl <Arg0, Arg1, Arg2, Res> Expectation3<Arg0, Arg1, Arg2, Res> {
    fn check(self, arg0: &Arg0, arg1: &Arg1, arg2: &Arg2) -> Res {
        self.call_match.arg0.matches(arg0).unwrap();
        self.call_match.arg1.matches(arg1).unwrap();
        self.call_match.arg2.matches(arg2).unwrap();
        self.result.get()
    }
}
impl<Arg0, Arg1, Arg2, Res> CheckCall for Expectation3<Arg0, Arg1, Arg2, Res> {
    fn check_call(self: Box<Self>, args: *const u8) -> *mut u8 {
        let args_tuple: &(Arg0, Arg1, Arg2) = unsafe { std::mem::transmute(args) };
        let result = self.check(&args_tuple.0, &args_tuple.1, &args_tuple.2);
        Box::into_raw(Box::new(result)) as *mut u8
    }
    fn get_mock_id(&self) -> usize { self.call_match.mock_id }
    fn get_method_name(&self) -> &'static str { self.call_match.method_name }
    fn describe(&self) -> String {
        format!("{}({}, {}, {})", self.get_method_name(),
                                  self.call_match.arg0.describe(),
                                  self.call_match.arg1.describe(),
                                  self.call_match.arg2.describe())
    }
}
impl<Arg0, Arg1, Arg2, Res> CallMatch3<Arg0, Arg1, Arg2, Res> {
    pub fn and_return(self, result: Res) -> Expectation3<Arg0, Arg1, Arg2, Res> {
        Expectation3 { call_match: self, result: MockCallResult::Return(result) }
    }

    pub fn and_panic(self, msg: String) -> Expectation3<Arg0, Arg1, Arg2, Res> {
        Expectation3 { call_match: self, result: MockCallResult::Panic(msg) }
    }
}

#[must_use]
pub struct CallMatch4<Arg0, Arg1, Arg2, Arg3, Res> {
    mock_id: usize,
    method_name: &'static str,
    arg0: Box<MatchArg<Arg0>>,
    arg1: Box<MatchArg<Arg1>>,
    arg2: Box<MatchArg<Arg2>>,
    arg3: Box<MatchArg<Arg3>>,

    _phantom: PhantomData<Res>,
}
impl<Arg0, Arg1, Arg2, Arg3, Res> CallMatch4<Arg0, Arg1, Arg2, Arg3, Res> {
    pub fn new(mock_id: usize, method_name: &'static str,
               arg0: Box<MatchArg<Arg0>>,
               arg1: Box<MatchArg<Arg1>>,
               arg2: Box<MatchArg<Arg2>>,
               arg3: Box<MatchArg<Arg3>>) -> Self {
        CallMatch4 {
            mock_id: mock_id,
            method_name: method_name,
            arg0: arg0,
            arg1: arg1,
            arg2: arg2,
            arg3: arg3,
            _phantom: PhantomData
        }
    }
}

#[must_use]
pub struct Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
    call_match: CallMatch4<Arg0, Arg1, Arg2, Arg3, Res>,
    result: MockCallResult<Res>,
}
impl <Arg0, Arg1, Arg2, Arg3, Res> Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
    fn check(self, arg0: &Arg0, arg1: &Arg1, arg2: &Arg2, arg3: &Arg3) -> Res {
        self.call_match.arg0.matches(arg0).unwrap();
        self.call_match.arg1.matches(arg1).unwrap();
        self.call_match.arg2.matches(arg2).unwrap();
        self.call_match.arg3.matches(arg3).unwrap();
        self.result.get()
    }
}
impl<Arg0, Arg1, Arg2, Arg3, Res> CheckCall for Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
    fn check_call(self: Box<Self>, args: *const u8) -> *mut u8 {
        let args_tuple: &(Arg0, Arg1, Arg2, Arg3) = unsafe { std::mem::transmute(args) };
        let result = self.check(&args_tuple.0, &args_tuple.1, &args_tuple.2, &args_tuple.3);
        Box::into_raw(Box::new(result)) as *mut u8
    }
    fn get_mock_id(&self) -> usize { self.call_match.mock_id }
    fn get_method_name(&self) -> &'static str { self.call_match.method_name }
    fn describe(&self) -> String {
        format!("{}({}, {}, {}, {})", self.get_method_name(),
                                      self.call_match.arg0.describe(),
                                      self.call_match.arg1.describe(),
                                      self.call_match.arg2.describe(),
                                      self.call_match.arg3.describe())
    }
}
impl<Arg0, Arg1, Arg2, Arg3, Res> CallMatch4<Arg0, Arg1, Arg2, Arg3, Res> {
    pub fn and_return(self, result: Res) -> Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
        Expectation4 { call_match: self, result: MockCallResult::Return(result) }
    }

    pub fn and_panic(self, msg: String) -> Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
        Expectation4 { call_match: self, result: MockCallResult::Panic(msg) }
    }
}


/// Argument matcher
///
/// Basically it is predicate telling whether argument
/// value satisfies to some criteria. However, in case
/// of mismatch it explains what and why doesn't match.
pub trait MatchArg<T> {
    fn matches(&self, arg: &T) -> Result<(), String>;
    fn describe(&self) -> String;
}

/// Matches argument with value of same type using equality.
impl<T: Eq + std::fmt::Debug> MatchArg<T> for T {
    fn matches(&self, arg: &T) -> Result<(), String> {
        if self == arg {
            Ok(())
        } else {
            Err(format!("{:?} is not equal to {:?}", arg, self))
        }
    }

    fn describe(&self) -> String {
        format!("{:?}", self)
    }
}

pub trait Mock {
    fn new(id: usize, scenario_int: Rc<RefCell<ScenarioInternals>>) -> Self;
}

pub trait Mocked {
    type MockImpl: Mock;
}

pub struct ScenarioInternals {
    events: Vec<Box<CheckCall>>,
}

pub struct Scenario {
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

    pub fn create_mock<T: ?Sized>(&mut self) -> <&'static T as Mocked>::MockImpl
            where &'static T: Mocked {
        <&'static T as Mocked>::MockImpl::new(self.get_next_mock_id(), self.internals.clone())
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

impl Drop for Scenario {
    fn drop(&mut self) {
        let events = &self.internals.borrow().events;
        if !events.is_empty() {
            let mut s = String::from("Expected calls are not performed:\n");
            for event in events {
                s.push_str(&format!("`{}`\n", event.describe()));
            }
            panic!(s);
        }
    }
}

impl ScenarioInternals {
    pub fn call(&mut self, mock_id: usize, method_name: &'static str, args_ptr: *const u8) -> *mut u8 {
        if self.events.is_empty() {
            panic!("Unexpected call of `{}`, no calls are expected", method_name);
        }
        let event = self.events.remove(0);
        if event.get_mock_id() != mock_id || event.get_method_name() != method_name {
            panic!("Unexpected call of `{}`, `{}` call is expected",
                   method_name, event.describe());
        }
        event.check_call(args_ptr)
    }
}
