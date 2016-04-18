use std::marker::PhantomData;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

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

pub trait Expectation {
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
impl<Res> Expectation for Expectation0<Res> {
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
impl<Arg0, Res> Expectation for Expectation1<Arg0, Res> {
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
impl<Arg0, Arg1, Res> Expectation for Expectation2<Arg0, Arg1, Res> {
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
impl<Arg0, Arg1, Arg2, Res> Expectation for Expectation3<Arg0, Arg1, Arg2, Res> {
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
impl<Arg0, Arg1, Arg2, Arg3, Res> Expectation for Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
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
        if !expectations.is_empty() {
            let mut s = String::from("Expected calls are not performed:\n");
            for expectation in expectations {
                let mock_name = int.mock_names.get(&expectation.get_mock_id()).unwrap();
                s.push_str(&format!("`{}::{}`\n", mock_name, expectation.describe()));
            }
            panic!(s);
        }
    }
}

impl ScenarioInternals {
    /// Verify call performed on mock object
    pub fn call(&mut self, mock_id: usize, method_name: &'static str, args_ptr: *const u8) -> *mut u8 {
        if self.expectations.is_empty() {
            let mock_name = self.mock_names.get(&mock_id).unwrap();
            panic!("Unexpected call of `{}::{}`, no calls are expected", mock_name, method_name);
        }
        let expectation = self.expectations.remove(0);
        if expectation.get_mock_id() != mock_id || expectation.get_method_name() != method_name {
            let expected_mock_name = self.mock_names.get(&expectation.get_mock_id()).unwrap();
            let actual_mock_name = self.mock_names.get(&mock_id).unwrap();
            panic!("Unexpected call of `{}::{}`, `{}::{}` call is expected",
                   actual_mock_name, method_name, expected_mock_name, expectation.describe());
        }
        expectation.check_call(args_ptr)
    }
}
