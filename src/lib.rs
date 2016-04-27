use std::marker::PhantomData;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

pub mod matchers;

enum MockCallResult0<T> {
    Return(T),
    Panic(String),
    Call(Box<Fn() -> T>),
}
impl<T> MockCallResult0<T> {
    fn get(self) -> T {
        match self {
            MockCallResult0::Return(value) => value,
            MockCallResult0::Panic(msg) => panic!(msg),
            MockCallResult0::Call(func) => func(),
        }
    }
}

enum MockCallResult1<Arg0, T> {
    Return(T),
    Panic(String),
    Call(Box<Fn(&Arg0) -> T>),
}
impl<Arg0, T> MockCallResult1<Arg0, T> {
    fn get(self, arg0: &Arg0) -> T {
        match self {
            MockCallResult1::Return(value) => value,
            MockCallResult1::Panic(msg) => panic!(msg),
            MockCallResult1::Call(func) => func(arg0),
        }
    }
}

enum MockCallResult2<Arg0, Arg1, T> {
    Return(T),
    Panic(String),
    Call(Box<Fn(&Arg0, &Arg1) -> T>),
}
impl<Arg0, Arg1, T> MockCallResult2<Arg0, Arg1, T> {
    fn get(self, arg0: &Arg0, arg1: &Arg1) -> T {
        match self {
            MockCallResult2::Return(value) => value,
            MockCallResult2::Panic(msg) => panic!(msg),
            MockCallResult2::Call(func) => func(arg0, arg1),
        }
    }
}

enum MockCallResult3<Arg0, Arg1, Arg2, T> {
    Return(T),
    Panic(String),
    Call(Box<Fn(&Arg0, &Arg1, &Arg2) -> T>),
}
impl<Arg0, Arg1, Arg2, T> MockCallResult3<Arg0, Arg1, Arg2, T> {
    fn get(self, arg0: &Arg0, arg1: &Arg1, arg2: &Arg2) -> T {
        match self {
            MockCallResult3::Return(value) => value,
            MockCallResult3::Panic(msg) => panic!(msg),
            MockCallResult3::Call(func) => func(arg0, arg1, arg2),
        }
    }
}

enum MockCallResult4<Arg0, Arg1, Arg2, Arg3, T> {
    Return(T),
    Panic(String),
    Call(Box<Fn(&Arg0, &Arg1, &Arg2, &Arg3) -> T>),
}
impl<Arg0, Arg1, Arg2, Arg3, T> MockCallResult4<Arg0, Arg1, Arg2, Arg3, T> {
    fn get(self, arg0: &Arg0, arg1: &Arg1, arg2: &Arg2, arg3: &Arg3) -> T {
        match self {
            MockCallResult4::Return(value) => value,
            MockCallResult4::Panic(msg) => panic!(msg),
            MockCallResult4::Call(func) => func(arg0, arg1, arg2, arg3),
        }
    }
}

pub trait Expectation {
    fn matches(&self, call: &Call) -> bool;
    fn matches_target(&self, call: &Call) -> bool;
    fn is_satisfied(&self) -> bool;
    fn satisfy(&mut self, call: &Call) -> *mut u8;
    fn validate(&self, call: &Call) -> Vec<Result<(), String>>;
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
    result: Option<MockCallResult0<Res>>,
}
impl<Res> Expectation for Expectation0<Res> {
    fn matches_target(&self, call: &Call) -> bool {
        self.call_match.mock_id == call.mock_id &&
        self.call_match.method_name == call.method_name
    }
    fn matches(&self, call: &Call) -> bool {
        self.matches_target(call)
    }
    fn is_satisfied(&self) -> bool {
        self.result.is_none()
    }
    fn satisfy(&mut self, _call: &Call) -> *mut u8 {
        let result = self.result.take().unwrap();
        Box::into_raw(Box::new(result.get())) as *mut u8
    }
    fn validate(&self, _call: &Call) -> Vec<Result<(), String>> {
        vec![]
    }
    fn get_mock_id(&self) -> usize { self.call_match.mock_id }
    fn get_method_name(&self) -> &'static str { self.call_match.method_name }
    fn describe(&self) -> String {
        format!("{}()", self.get_method_name())
    }
}
impl<Res> CallMatch0<Res> {
    pub fn and_return(self, result: Res) -> Expectation0<Res> {
        Expectation0 { call_match: self, result: Some(MockCallResult0::Return(result)) }
    }

    pub fn and_panic(self, msg: String) -> Expectation0<Res> {
        Expectation0 { call_match: self, result: Some(MockCallResult0::Panic(msg)) }
    }

    pub fn and_call<F>(self, func: F) -> Expectation0<Res>
            where F: Fn() -> Res + 'static {
        Expectation0 { call_match: self, result: Some(MockCallResult0::Call(Box::new(func))) }
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
    result: Option<MockCallResult1<Arg0, Res>>,
}
impl<Arg0, Res> Expectation1<Arg0, Res> {
    fn get_args(call: &Call) -> &(Arg0,) {
        unsafe { std::mem::transmute(call.args_ptr) }
    }
}
impl<Arg0, Res> Expectation for Expectation1<Arg0, Res> {
    fn matches_target(&self, call: &Call) -> bool {
        self.call_match.mock_id == call.mock_id &&
        self.call_match.method_name == call.method_name
    }
    fn matches(&self, call: &Call) -> bool {
        if !self.matches_target(call) {
            return false;
        }

        let args = Self::get_args(call);
        self.call_match.arg0.matches(&args.0).is_ok()
    }
    fn is_satisfied(&self) -> bool {
        self.result.is_none()
    }
    fn satisfy(&mut self, call: &Call) -> *mut u8 {
        let result = self.result.take().unwrap();
        let args = Self::get_args(call);
        Box::into_raw(Box::new(result.get(&args.0))) as *mut u8
    }
    fn validate(&self, call: &Call) -> Vec<Result<(), String>> {
        let args = Self::get_args(call);
        vec![ self.call_match.arg0.matches(&args.0) ]
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
        Expectation1 { call_match: self, result: Some(MockCallResult1::Return(result)) }
    }

    pub fn and_panic(self, msg: String) -> Expectation1<Arg0, Res> {
        Expectation1 { call_match: self, result: Some(MockCallResult1::Panic(msg)) }
    }

    pub fn and_call<F>(self, func: F) -> Expectation1<Arg0, Res>
            where F: Fn(&Arg0) -> Res + 'static {
        Expectation1 { call_match: self, result: Some(MockCallResult1::Call(Box::new(func))) }
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
    result: Option<MockCallResult2<Arg0, Arg1, Res>>,
}
impl <Arg0, Arg1, Res> Expectation2<Arg0, Arg1, Res> {
    fn get_args(call: &Call) -> &(Arg0, Arg1) {
        unsafe { std::mem::transmute(call.args_ptr) }
    }
}
impl<Arg0, Arg1, Res> Expectation for Expectation2<Arg0, Arg1, Res> {
    fn matches_target(&self, call: &Call) -> bool {
        self.call_match.mock_id == call.mock_id &&
        self.call_match.method_name == call.method_name
    }
    fn matches(&self, call: &Call) -> bool {
        if !self.matches_target(call) {
            return false;
        }

        let args = Self::get_args(call);
        self.call_match.arg0.matches(&args.0).is_ok() &&
        self.call_match.arg1.matches(&args.1).is_ok()
    }
    fn is_satisfied(&self) -> bool {
        self.result.is_none()
    }
    fn satisfy(&mut self, call: &Call) -> *mut u8 {
        let result = self.result.take().unwrap();
        let args = Self::get_args(call);
        Box::into_raw(Box::new(result.get(&args.0, &args.1))) as *mut u8
    }
    fn validate(&self, call: &Call) -> Vec<Result<(), String>> {
        let args = Self::get_args(call);
        vec![ self.call_match.arg0.matches(&args.0),
              self.call_match.arg1.matches(&args.1) ]
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
        Expectation2 { call_match: self, result: Some(MockCallResult2::Return(result)) }
    }

    pub fn and_panic(self, msg: String) -> Expectation2<Arg0, Arg1, Res> {
        Expectation2 { call_match: self, result: Some(MockCallResult2::Panic(msg)) }
    }

    pub fn and_call<F>(self, func: F) -> Expectation2<Arg0, Arg1, Res>
            where F: Fn(&Arg0, &Arg1) -> Res + 'static {
        Expectation2 { call_match: self, result: Some(MockCallResult2::Call(Box::new(func))) }
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
    result: Option<MockCallResult3<Arg0, Arg1, Arg2, Res>>,
}
impl <Arg0, Arg1, Arg2, Res> Expectation3<Arg0, Arg1, Arg2, Res> {
    fn get_args(call: &Call) -> &(Arg0, Arg1, Arg2) {
        unsafe { std::mem::transmute(call.args_ptr) }
    }
}
impl<Arg0, Arg1, Arg2, Res> Expectation for Expectation3<Arg0, Arg1, Arg2, Res> {
    fn matches_target(&self, call: &Call) -> bool {
        self.call_match.mock_id == call.mock_id &&
        self.call_match.method_name == call.method_name
    }
    fn matches(&self, call: &Call) -> bool {
        if !self.matches_target(call) {
            return false;
        }

        let args = Self::get_args(call);
        self.call_match.arg0.matches(&args.0).is_ok() &&
        self.call_match.arg1.matches(&args.1).is_ok() &&
        self.call_match.arg2.matches(&args.2).is_ok()
    }
    fn is_satisfied(&self) -> bool {
        self.result.is_none()
    }
    fn satisfy(&mut self, call: &Call) -> *mut u8 {
        let result = self.result.take().unwrap();
        let args = Self::get_args(call);
        Box::into_raw(Box::new(result.get(&args.0, &args.1, &args.2))) as *mut u8
    }
    fn validate(&self, call: &Call) -> Vec<Result<(), String>> {
        let args = Self::get_args(call);
        vec![ self.call_match.arg0.matches(&args.0),
              self.call_match.arg1.matches(&args.1),
              self.call_match.arg2.matches(&args.2) ]
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
        Expectation3 { call_match: self, result: Some(MockCallResult3::Return(result)) }
    }

    pub fn and_panic(self, msg: String) -> Expectation3<Arg0, Arg1, Arg2, Res> {
        Expectation3 { call_match: self, result: Some(MockCallResult3::Panic(msg)) }
    }

    pub fn and_call<F>(self, func: F) -> Expectation3<Arg0, Arg1, Arg2, Res>
            where F: Fn(&Arg0, &Arg1, &Arg2) -> Res + 'static {
        Expectation3 { call_match: self, result: Some(MockCallResult3::Call(Box::new(func))) }
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
    result: Option<MockCallResult4<Arg0, Arg1, Arg2, Arg3, Res>>,
}
impl <Arg0, Arg1, Arg2, Arg3, Res> Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
    fn get_args(call: &Call) -> &(Arg0, Arg1, Arg2, Arg3) {
        unsafe { std::mem::transmute(call.args_ptr) }
    }
}
impl<Arg0, Arg1, Arg2, Arg3, Res> Expectation for Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
    fn matches_target(&self, call: &Call) -> bool {
        self.call_match.mock_id == call.mock_id &&
        self.call_match.method_name == call.method_name
    }
    fn matches(&self, call: &Call) -> bool {
        if !self.matches_target(call) {
            return false;
        }

        let args = Self::get_args(call);
        self.call_match.arg0.matches(&args.0).is_ok() &&
        self.call_match.arg1.matches(&args.1).is_ok() &&
        self.call_match.arg2.matches(&args.2).is_ok() &&
        self.call_match.arg3.matches(&args.3).is_ok()
    }
    fn is_satisfied(&self) -> bool {
        self.result.is_none()
    }
    fn satisfy(&mut self, call: &Call) -> *mut u8 {
        let result = self.result.take().unwrap();
        let args = Self::get_args(call);
        Box::into_raw(Box::new(result.get(&args.0, &args.1,
                                          &args.2, &args.3))) as *mut u8
    }
    fn validate(&self, call: &Call) -> Vec<Result<(), String>> {
        let args = Self::get_args(call);
        vec![ self.call_match.arg0.matches(&args.0),
              self.call_match.arg1.matches(&args.1),
              self.call_match.arg2.matches(&args.2),
              self.call_match.arg3.matches(&args.3) ]
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
        Expectation4 { call_match: self, result: Some(MockCallResult4::Return(result)) }
    }

    pub fn and_panic(self, msg: String) -> Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
        Expectation4 { call_match: self, result: Some(MockCallResult4::Panic(msg)) }
    }

    pub fn and_call<F>(self, func: F) -> Expectation4<Arg0, Arg1, Arg2, Arg3, Res>
            where F: Fn(&Arg0, &Arg1, &Arg2, &Arg3) -> Res + 'static {
        Expectation4 { call_match: self, result: Some(MockCallResult4::Call(Box::new(func))) }
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

    /// Returns name of mocked class
    fn class_name() -> &'static str;
}

pub struct ScenarioInternals {
    expectations: Vec<Box<Expectation>>,

    /// Mapping from mock ID to mock name.
    mock_names: HashMap<usize, Rc<String>>,
    /// Set of used mock names used to quicly check for conflicts.
    allocated_names: HashSet<Rc<String>>,
}

pub struct Scenario {
    internals: Rc<RefCell<ScenarioInternals>>,
    next_mock_id: usize,
}

impl Scenario {
    pub fn new() -> Self {
        Scenario {
            internals: Rc::new(RefCell::new(ScenarioInternals {
                expectations: Vec::new(),

                mock_names: HashMap::new(),
                allocated_names: HashSet::new(),
            })),
            next_mock_id: 0,
        }
    }

    pub fn create_mock<T: ?Sized>(&mut self) -> <&'static T as Mocked>::MockImpl
            where &'static T: Mocked {
        let mock_id = self.get_next_mock_id();
        self.generate_name_for_class(mock_id, <&'static T as Mocked>::class_name());
        <&'static T as Mocked>::MockImpl::new(mock_id, self.internals.clone())
    }

    pub fn create_named_mock<T: ?Sized>(&mut self, name: String) -> <&'static T as Mocked>::MockImpl
            where &'static T: Mocked {
        let mock_id = self.get_next_mock_id();
        self.register_name(mock_id, name);
        <&'static T as Mocked>::MockImpl::new(mock_id, self.internals.clone())
    }

    fn get_next_mock_id(&mut self) -> usize {
        let id = self.next_mock_id;
        self.next_mock_id += 1;
        id
    }

    pub fn expect<C: Expectation + 'static>(&mut self, call: C) {
        self.internals.borrow_mut().expectations.push(Box::new(call));
    }

    fn register_name(&mut self, mock_id: usize, name: String) {
        let mut int = self.internals.borrow_mut();
        if int.allocated_names.contains(&name) {
            panic!("Mock name {} already used", name);
        }
        let name_rc = Rc::new(name);
        int.mock_names.insert(mock_id, name_rc.clone());
        int.allocated_names.insert(name_rc);
    }

    fn generate_name_for_class(&mut self, mock_id: usize, class_name: &str) {
        let mut int = self.internals.borrow_mut();
        for i in 0.. {
            let name = format!("{}#{}", class_name, i);
            if !int.allocated_names.contains(&name) {
                let name_rc = Rc::new(name);
                int.mock_names.insert(mock_id, name_rc.clone());
                int.allocated_names.insert(name_rc);
                break;
            }
        }
    }
}

impl Drop for Scenario {
    fn drop(&mut self) {
        // Test is already failed, so it isn't necessary to check remaining
        // expectations. And if we do, then panic-during-drop will cause
        // test to fail with uncomprehensive message like:
        // "(signal: 4, SIGILL: illegal instruction)"
        if std::thread::panicking() {
            return;
        }

        let int = self.internals.borrow();
        let expectations = &int.expectations;
        let mock_names = &int.mock_names;
        let mut active_expectations = expectations.iter().filter(|e| !e.is_satisfied()).peekable();
        if active_expectations.peek().is_some() {
            let mut s = String::from("Expected calls are not performed:\n");
            for expectation in active_expectations {
                let mock_name = mock_names.get(&expectation.get_mock_id()).unwrap();
                s.push_str(&format!("`{}::{}`\n", mock_name, expectation.describe()));
            }
            panic!(s);
        }
    }
}

pub struct Call {
    mock_id: usize,
    method_name: &'static str,
    args_ptr: *const u8,
}

impl ScenarioInternals {
    /// Verify call performed on mock object
    pub fn call(&mut self, mock_id: usize, method_name: &'static str, args_ptr: *const u8) -> *mut u8 {
        if self.expectations.is_empty() {
            let mock_name = self.mock_names.get(&mock_id).unwrap();
            panic!("\nUnexpected call to `{}::{}`, no calls are expected", mock_name, method_name);
        }

        let call = Call { mock_id: mock_id, method_name: method_name, args_ptr: args_ptr };

        for expectation in self.expectations.iter_mut().rev() {
            if expectation.matches(&call) {
                if expectation.is_satisfied() {
                    let mock_name = self.mock_names.get(&mock_id).unwrap();
                    panic!("Call to `{}::{}` is already performed", mock_name, expectation.describe());
                }

                return expectation.satisfy(&call);
            }
        }

        // No expectations exactly matching call are found. However this may be
        // because of unexpected argument values. So check active expectations
        // with matching target (i.e. mock and method) and validate arguments.
        let mut first_match = true;
        let mock_name = self.mock_names.get(&mock_id).unwrap();

        let mut msg = String::new();
        msg.push_str(&format!("\nUnexpected call to `{}.{}`\n\n", mock_name, method_name));
        for expectation in self.expectations.iter().rev() {
            if !expectation.is_satisfied() && expectation.matches_target(&call) {
                if first_match {
                    msg.push_str(&format!("Here are active expectations for same method call:\n"));
                    first_match = false;
                }

                msg.push_str(&format!("\n  Expectation `{}.{}`:\n", mock_name, expectation.describe()));
                for (index, res) in expectation.validate(&call).iter().enumerate() {
                    match res {
                        &Err(ref err) => msg.push_str(&format!("    Arg #{}: {}\n", index, err)),
                        &Ok(()) => ()
                    }
                }
            }
        }

        if first_match {
            msg.push_str(&format!("There are no active expectations for same method call\n"));
        }

        panic!(msg);
    }
}
