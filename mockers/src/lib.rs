#![feature(fnbox, specialization)]

use std::marker::PhantomData;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use std::fmt::Write;
use std::ops::DerefMut;

mod box_fn;
mod dbg;
pub mod matchers;
pub mod cardinality;
#[macro_use]
pub mod clone;

use crate::cardinality::{Cardinality, CardinalityCheckResult};
use crate::dbg::dbg;

thread_local! {
    // Mapping from mock_type_id of 'extern' block mock to corresponding mock object.
    // It is needed since mock is object but mocked functions are static.
    pub static EXTERN_MOCKS: RefCell<HashMap<usize, (usize, Rc<RefCell<ScenarioInternals>>)>> = RefCell::new(HashMap::new());
}

type Action0<T> = box_fn::BoxFn0<T>;
type ActionClone0<T> = Rc<RefCell<dyn FnMut() -> T>>;

type Action1<Arg0, T> = box_fn::BoxFn1<Arg0, T>;
type ActionClone1<Arg0, T> = Rc<RefCell<dyn FnMut(Arg0) -> T>>;

type Action2<Arg0, Arg1, T> = box_fn::BoxFn2<Arg0, Arg1, T>;
type ActionClone2<Arg0, Arg1, T> = Rc<RefCell<dyn FnMut(Arg0, Arg1) -> T>>;

type Action3<Arg0, Arg1, Arg2, T> = box_fn::BoxFn3<Arg0, Arg1, Arg2, T>;
type ActionClone3<Arg0, Arg1, Arg2, T> = Rc<RefCell<dyn FnMut(Arg0, Arg1, Arg2) -> T>>;

type Action4<Arg0, Arg1, Arg2, Arg3, T> = box_fn::BoxFn4<Arg0, Arg1, Arg2, Arg3, T>;
type ActionClone4<Arg0, Arg1, Arg2, Arg3, T> = Rc<RefCell<dyn FnMut(Arg0, Arg1, Arg2, Arg3) -> T>>;

pub trait CallMatch {
    fn matches_args(&self, call: &Call) -> bool;
    fn matches(&self, call: &Call) -> bool {
        self.matches_target(call) && self.matches_args(call)
    }
    fn matches_target(&self, call: &Call) -> bool {
        self.get_mock_id() == call.method_data.mock_id &&
        self.get_method_name() == call.method_data.method_name
    }
    fn matches_method(&self, call: &Call) -> bool {
        self.get_mock_type_id() == call.method_data.mock_type_id &&
        self.get_method_name() == call.method_data.method_name
    }
    fn validate(&self, call: &Call) -> Vec<Result<(), String>>;
    fn get_mock_id(&self) -> usize;
    fn get_mock_type_id(&self) -> usize;
    fn get_method_name(&self) -> &'static str;
    fn describe(&self) -> String;
}

pub trait Expectation {
    fn call_match(&self) -> &dyn CallMatch;
    fn is_satisfied(&self) -> bool;
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8>;
    fn describe(&self) -> String;
}

pub struct ExpectationNever<CM: CallMatch> {
    call_match: CM,
}
impl<CM: CallMatch> Expectation for ExpectationNever<CM> {
    fn call_match(&self) -> &dyn CallMatch {
        &self.call_match
    }
    fn is_satisfied(&self) -> bool {
        true
    }
    fn satisfy(&mut self, _call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        panic!("{}.{} should never be called", mock_name, self.call_match().get_method_name());
    }
    fn describe(&self) -> String {
        format!("{} should never be called", self.call_match.describe())
    }
}

#[must_use]
pub struct CallMatch0<Res> {
    mock_id: usize,
    mock_type_id: usize,
    method_name: &'static str,

    _phantom: PhantomData<Res>,
}
impl<Res> CallMatch0<Res> {
    pub fn new(mock_id: usize, mock_type_id: usize, method_name: &'static str) -> Self {
        CallMatch0 {
            mock_id: mock_id,
            mock_type_id: mock_type_id,
            method_name: method_name,
            _phantom: PhantomData
        }
    }

    fn get_args(mut call: Call) -> Box<()> {
        unsafe { Box::from_raw(call.take_args() as *mut ()) }
    }
}
impl<Res> CallMatch for CallMatch0<Res> {
    fn matches_args(&self, call: &Call) -> bool {
        assert!(call.method_data.mock_type_id == self.mock_type_id &&
                call.method_data.method_name == self.method_name);
        true
    }
    fn validate(&self, _call: &Call) -> Vec<Result<(), String>> {
        vec![]
    }
    fn get_mock_id(&self) -> usize { self.mock_id }
    fn get_mock_type_id(&self) -> usize { self.mock_type_id }
    fn get_method_name(&self) -> &'static str { self.method_name }
    fn describe(&self) -> String {
        format!("{}()", self.method_name)
    }
}

#[must_use]
pub struct Reaction0<Res> {
    call_match: CallMatch0<Res>,
    action: ActionClone0<Res>,
}
impl<Res> Reaction0<Res> {
    pub fn times<C: Cardinality + 'static>(self, cardinality: C) -> ExpectationTimes0<Res> {
        ExpectationTimes0::new(self.call_match, self.action, Box::new(cardinality))
    }
}

#[must_use]
pub struct ExpectationTimes0<Res> {
    action: ActionClone0<Res>,
    call_match: CallMatch0<Res>,
    cardinality: Box<dyn Cardinality>,
    count: u32,
}
impl<Res> ExpectationTimes0<Res> {
    fn new(call_match: CallMatch0<Res>, action: ActionClone0<Res>, cardinality: Box<dyn Cardinality>) -> Self {
        ExpectationTimes0 { call_match: call_match, action: action, cardinality: cardinality, count: 0 }
    }
}
impl<Res: 'static> Expectation for ExpectationTimes0<Res> {
    fn call_match(&self) -> &dyn CallMatch {
        &self.call_match
    }
    fn is_satisfied(&self) -> bool {
        self.cardinality.check(self.count) == CardinalityCheckResult::Satisfied
    }
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        self.count += 1;
        if self.cardinality.check(self.count) == CardinalityCheckResult::Wrong {
            panic!("{}.{} is called for the {} time, but expected to be {}",
                   mock_name, self.call_match().get_method_name(), format_ordinal(self.count),
                   self.cardinality.describe_upper_bound());
        }
        let _args = CallMatch0::<Res>::get_args(call);
        box_fn::BoxFn0::new({
            let action = self.action.clone();
            move || {
                let result = (action.borrow_mut().deref_mut())();
                Box::into_raw(Box::new(result)) as *mut u8
            }
        })
    }
    fn describe(&self) -> String {
        format!("{} must be {}, called {} times",
                self.call_match.describe(), self.cardinality.describe(), self.count)
    }
}

#[must_use]
pub struct Expectation0<Res> {
    call_match: CallMatch0<Res>,
    action: Option<Action0<Res>>,
}
impl<Res: 'static> Expectation for Expectation0<Res> {
    fn call_match(&self) -> &dyn CallMatch {
        &self.call_match
    }
    fn is_satisfied(&self) -> bool {
        self.action.is_none()
    }
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        match self.action.take() {
            Some(action) => {
                // nightly: let box () = CallMatch0::<Res>::get_args(call);
                let () = *CallMatch0::<Res>::get_args(call);
                box_fn::BoxFn0::<*mut u8>::new(move || {
                    let result = action.call();
                    Box::into_raw(Box::new(result)) as *mut u8
                })
            },
            None => {
                panic!("{}.{} was already called earlier", mock_name, self.call_match().get_method_name());
            }
        }
    }
    fn describe(&self) -> String {
        self.call_match.describe()
    }
}
impl<Res: 'static> CallMatch0<Res> {
    pub fn and_return(self, result: Res) -> Expectation0<Res> {
        Expectation0 { call_match: self, action: Some(Action0::new(move || result)) }
    }

    pub fn and_panic(self, msg: String) -> Expectation0<Res> {
        Expectation0 { call_match: self, action: Some(Action0::new(move || panic!(msg))) }
    }

    pub fn and_call<F>(self, func: F) -> Expectation0<Res>
            where F: FnOnce() -> Res + 'static {
        Expectation0 { call_match: self, action: Some(Action0::new(func)) }
    }

    pub fn never(self) -> ExpectationNever<Self> {
        ExpectationNever { call_match: self }
    }
}
impl<Res: Clone + 'static> CallMatch0<Res> {
    pub fn and_return_clone(self, result: Res) -> Reaction0<Res> {
        Reaction0 { call_match: self, action: Rc::new(RefCell::new(move || result.clone())) }
    }
}
impl<Res> CallMatch0<Res> {
    pub fn and_call_clone<F>(self, func: F) -> Reaction0<Res>
            where F: FnMut() -> Res + 'static {
        Reaction0 { call_match: self, action: Rc::new(RefCell::new(func)) }
    }
}

impl<Res: Default + 'static> CallMatch0<Res> {
    pub fn and_return_default(self) -> Reaction0<Res> {
        Reaction0 { call_match: self, action: Rc::new(RefCell::new(Res::default)) }
    }
}

#[must_use]
pub struct CallMatch1<Arg0, Res> {
    mock_id: usize,
    mock_type_id: usize,
    method_name: &'static str,
    arg0: Box<dyn MatchArg<Arg0>>,

    _phantom: PhantomData<Res>,
}
impl<Arg0, Res> CallMatch1<Arg0, Res> {
    pub fn new(mock_id: usize, mock_type_id: usize, method_name: &'static str, arg0: Box<dyn MatchArg<Arg0>>) -> Self {
        CallMatch1 {
            mock_id: mock_id,
            mock_type_id: mock_type_id,
            method_name: method_name,
            arg0: arg0,
            _phantom: PhantomData
        }
    }

    fn get_args_ref(call: &Call) -> &mut (Arg0,) {
        unsafe { &mut *(call.args_ptr as *mut (Arg0,)) }
    }

    fn get_args(mut call: Call) -> Box<(Arg0,)> {
        unsafe { Box::from_raw(call.take_args() as *mut (Arg0,)) }
    }
}
impl<Arg0, Res> CallMatch for CallMatch1<Arg0, Res> {
    fn matches_args(&self, call: &Call) -> bool {
        assert!(call.method_data.mock_type_id == self.mock_type_id &&
                call.method_data.method_name == self.method_name);

        let args = Self::get_args_ref(call);
        self.arg0.matches(&args.0).is_ok()
    }
    fn validate(&self, call: &Call) -> Vec<Result<(), String>> {
        let args = Self::get_args_ref(call);
        vec![ self.arg0.matches(&args.0) ]
    }
    fn get_mock_id(&self) -> usize { self.mock_id }
    fn get_mock_type_id(&self) -> usize { self.mock_type_id }
    fn get_method_name(&self) -> &'static str { self.method_name }
    fn describe(&self) -> String {
        format!("{}({})", self.get_method_name(),
                          self.arg0.describe())
    }
}

#[must_use]
pub struct Reaction1<Arg0, Res> {
    call_match: CallMatch1<Arg0, Res>,
    action: ActionClone1<Arg0, Res>,
}
impl<Arg0, Res> Reaction1<Arg0, Res> {
    pub fn times<C: Cardinality + 'static>(self, cardinality: C) -> ExpectationTimes1<Arg0, Res> {
        ExpectationTimes1::new(self.call_match, self.action, Box::new(cardinality))
    }
}

#[must_use]
pub struct ExpectationTimes1<Arg0, Res> {
    action: ActionClone1<Arg0, Res>,
    call_match: CallMatch1<Arg0, Res>,
    cardinality: Box<dyn Cardinality>,
    count: u32,
}
impl<Arg0, Res> ExpectationTimes1<Arg0, Res> {
    fn new(call_match: CallMatch1<Arg0, Res>, action: ActionClone1<Arg0, Res>, cardinality: Box<dyn Cardinality>) -> Self {
        ExpectationTimes1 { call_match: call_match, action: action, cardinality: cardinality, count: 0 }
    }
}
impl<Arg0: 'static, Res: 'static> Expectation for ExpectationTimes1<Arg0, Res> {
    fn call_match(&self) -> &dyn CallMatch {
        &self.call_match
    }
    fn is_satisfied(&self) -> bool {
        self.cardinality.check(self.count) == CardinalityCheckResult::Satisfied
    }
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        self.count += 1;
        if self.cardinality.check(self.count) == CardinalityCheckResult::Wrong {
            panic!("{}.{} is called for the {} time, but expected to be {}",
                   mock_name, self.call_match().get_method_name(), format_ordinal(self.count),
                   self.cardinality.describe_upper_bound());
        }
        // nightly: let box (arg0,) = CallMatch1::<Arg0, Res>::get_args(call);
        let (arg0,) = *CallMatch1::<Arg0, Res>::get_args(call);
        box_fn::BoxFn0::new({
            let action = self.action.clone();
            move || {
                let result = action.borrow_mut().deref_mut()(arg0);
                Box::into_raw(Box::new(result)) as *mut u8
            }
        })
    }
    fn describe(&self) -> String {
        format!("{} must be {}, called {} times",
                self.call_match.describe(), self.cardinality.describe(), self.count)
    }
}

#[must_use]
pub struct Expectation1<Arg0, Res> {
    call_match: CallMatch1<Arg0, Res>,
    action: Option<Action1<Arg0, Res>>,
}
impl<Arg0: 'static, Res: 'static> Expectation for Expectation1<Arg0, Res> {
    fn call_match(&self) -> &dyn CallMatch {
        &self.call_match
    }
    fn is_satisfied(&self) -> bool {
        self.action.is_none()
    }
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        match self.action.take() {
            Some(action) => {
                // nightly: let box (arg0,) = CallMatch1::<Arg0, Res>::get_args(call);
                let (arg0,) = *CallMatch1::<Arg0, Res>::get_args(call);
                box_fn::BoxFn0::new(move || {
                    let result = action.call(arg0);
                    Box::into_raw(Box::new(result)) as *mut u8
                })
            },
            None => {
                panic!("{}.{} was already called earlier", mock_name, self.call_match().get_method_name());
            }
        }
    }
    fn describe(&self) -> String {
        self.call_match.describe()
    }
}
impl<Arg0, Res: 'static> CallMatch1<Arg0, Res> {
    pub fn and_return(self, result: Res) -> Expectation1<Arg0, Res> {
        Expectation1 { call_match: self, action: Some(Action1::new(move |_| result)) }
    }

    pub fn and_panic(self, msg: String) -> Expectation1<Arg0, Res> {
        Expectation1 { call_match: self, action: Some(Action1::new(move |_| panic!(msg))) }
    }

    pub fn and_call<F>(self, func: F) -> Expectation1<Arg0, Res>
            where F: FnOnce(Arg0) -> Res + 'static {
        Expectation1 { call_match: self, action: Some(Action1::new(func)) }
    }

    pub fn never(self) -> ExpectationNever<Self> {
        ExpectationNever { call_match: self }
    }
}
impl<Arg0, Res: Clone + 'static> CallMatch1<Arg0, Res> {
    pub fn and_return_clone(self, result: Res) -> Reaction1<Arg0, Res> {
        Reaction1 { call_match: self, action: Rc::new(RefCell::new(move |_| result.clone())) }
    }
}
impl<Arg0, Res: 'static> CallMatch1<Arg0, Res> {
    pub fn and_call_clone<F>(self, func: F) -> Reaction1<Arg0, Res>
            where F: FnMut(Arg0) -> Res + 'static {
        Reaction1 { call_match: self, action: Rc::new(RefCell::new(func)) }
    }
}
impl<Arg0, Res: Default + 'static> CallMatch1<Arg0, Res> {
    pub fn and_return_default(self) -> Reaction1<Arg0, Res> {
        Reaction1 { call_match: self, action: Rc::new(RefCell::new(|_| Res::default())) }
    }
}

#[must_use]
pub struct CallMatch2<Arg0, Arg1, Res> {
    mock_id: usize,
    mock_type_id: usize,
    method_name: &'static str,
    arg0: Box<dyn MatchArg<Arg0>>,
    arg1: Box<dyn MatchArg<Arg1>>,

    _phantom: PhantomData<Res>,
}
impl<Arg0, Arg1, Res> CallMatch2<Arg0, Arg1, Res> {
    pub fn new(mock_id: usize, mock_type_id: usize, method_name: &'static str,
               arg0: Box<dyn MatchArg<Arg0>>,
               arg1: Box<dyn MatchArg<Arg1>>) -> Self {
        CallMatch2 {
            mock_id: mock_id,
            mock_type_id: mock_type_id,
            method_name: method_name,
            arg0: arg0,
            arg1: arg1,
            _phantom: PhantomData
        }
    }

    fn get_args_ref(call: &Call) -> &mut (Arg0, Arg1) {
        unsafe { &mut *(call.args_ptr as *mut (Arg0, Arg1)) }
    }

    fn get_args(mut call: Call) -> Box<(Arg0, Arg1)> {
        unsafe { Box::from_raw(call.take_args() as *mut (Arg0, Arg1)) }
    }
}
impl<Arg0, Arg1, Res> CallMatch for CallMatch2<Arg0, Arg1, Res> {
    fn matches_args(&self, call: &Call) -> bool {
        assert!(call.method_data.mock_type_id == self.mock_type_id &&
                call.method_data.method_name == self.method_name);

        let args = Self::get_args_ref(call);
        self.arg0.matches(&args.0).is_ok() &&
        self.arg1.matches(&args.1).is_ok()
    }
    fn validate(&self, call: &Call) -> Vec<Result<(), String>> {
        let args = Self::get_args_ref(call);
        vec![ self.arg0.matches(&args.0),
              self.arg1.matches(&args.1) ]
    }
    fn get_mock_id(&self) -> usize { self.mock_id }
    fn get_mock_type_id(&self) -> usize { self.mock_type_id }
    fn get_method_name(&self) -> &'static str { self.method_name }
    fn describe(&self) -> String {
        format!("{}({}, {})", self.get_method_name(),
                              self.arg0.describe(),
                              self.arg1.describe())
    }
}

#[must_use]
pub struct Reaction2<Arg0, Arg1, Res> {
    call_match: CallMatch2<Arg0, Arg1, Res>,
    action: ActionClone2<Arg0, Arg1, Res>,
}
impl<Arg0, Arg1, Res> Reaction2<Arg0, Arg1, Res> {
    pub fn times<C: Cardinality + 'static>(self, cardinality: C) -> ExpectationTimes2<Arg0, Arg1, Res> {
        ExpectationTimes2::new(self.call_match, self.action, Box::new(cardinality))
    }
}

#[must_use]
pub struct ExpectationTimes2<Arg0, Arg1, Res> {
    action: ActionClone2<Arg0, Arg1, Res>,
    call_match: CallMatch2<Arg0, Arg1, Res>,
    cardinality: Box<dyn Cardinality>,
    count: u32,
}
impl<Arg0, Arg1, Res> ExpectationTimes2<Arg0, Arg1, Res> {
    fn new(call_match: CallMatch2<Arg0, Arg1, Res>, action: ActionClone2<Arg0, Arg1, Res>, cardinality: Box<dyn Cardinality>) -> Self {
        ExpectationTimes2 { call_match: call_match, action: action, cardinality: cardinality, count: 0 }
    }
}
impl<Arg0: 'static, Arg1: 'static, Res: 'static> Expectation for ExpectationTimes2<Arg0, Arg1, Res> {
    fn call_match(&self) -> &dyn CallMatch {
        &self.call_match
    }
    fn is_satisfied(&self) -> bool {
        self.cardinality.check(self.count) == CardinalityCheckResult::Satisfied
    }
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        self.count += 1;
        if self.cardinality.check(self.count) == CardinalityCheckResult::Wrong {
            panic!("{}.{} is called for the {} time, but expected to be {}",
                   mock_name, self.call_match().get_method_name(), format_ordinal(self.count),
                   self.cardinality.describe_upper_bound());
        }
        // nightly: let box (arg0, arg1) = CallMatch2::<Arg0, Arg1, Res>::get_args(call);
        let (arg0, arg1) = *CallMatch2::<Arg0, Arg1, Res>::get_args(call);
        box_fn::BoxFn0::new({
            let action = self.action.clone();
            move || {
                let result = action.borrow_mut().deref_mut()(arg0, arg1);
                Box::into_raw(Box::new(result)) as *mut u8
            }
        })
    }
    fn describe(&self) -> String {
        format!("{} must be {}, called {} times",
                self.call_match.describe(), self.cardinality.describe(), self.count)
    }
}

#[must_use]
pub struct Expectation2<Arg0, Arg1, Res> {
    call_match: CallMatch2<Arg0, Arg1, Res>,
    action: Option<Action2<Arg0, Arg1, Res>>,
}
impl<Arg0: 'static, Arg1: 'static, Res: 'static> Expectation for Expectation2<Arg0, Arg1, Res> {
    fn call_match(&self) -> &dyn CallMatch {
        &self.call_match
    }
    fn is_satisfied(&self) -> bool {
        self.action.is_none()
    }
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        match self.action.take() {
            Some(action) => {
                // nightly: let box (arg0, arg1) = CallMatch2::<Arg0, Arg1, Res>::get_args(call);
                let (arg0, arg1) = *CallMatch2::<Arg0, Arg1, Res>::get_args(call);
                box_fn::BoxFn0::new(move || {
                    let result = action.call(arg0, arg1);
                    Box::into_raw(Box::new(result)) as *mut u8
                })
            },
            None => {
                panic!("{}.{} was already called earlier", mock_name, self.call_match().get_method_name());
            }
        }
    }
    fn describe(&self) -> String {
        self.call_match.describe()
    }
}
impl<Arg0, Arg1, Res: 'static> CallMatch2<Arg0, Arg1, Res> {
    pub fn and_return(self, result: Res) -> Expectation2<Arg0, Arg1, Res> {
        Expectation2 { call_match: self, action: Some(Action2::new(move |_, _| result)) }
    }

    pub fn and_panic(self, msg: String) -> Expectation2<Arg0, Arg1, Res> {
        Expectation2 { call_match: self, action: Some(Action2::new(move |_, _| panic!(msg))) }
    }

    pub fn and_call<F>(self, func: F) -> Expectation2<Arg0, Arg1, Res>
            where F: FnOnce(Arg0, Arg1) -> Res + 'static {
        Expectation2 { call_match: self, action: Some(Action2::new(func)) }
    }

    pub fn never(self) -> ExpectationNever<Self> {
        ExpectationNever { call_match: self }
    }
}
impl<Arg0, Arg1, Res: Clone + 'static> CallMatch2<Arg0, Arg1, Res> {
    pub fn and_return_clone(self, result: Res) -> Reaction2<Arg0, Arg1, Res> {
        Reaction2 { call_match: self, action: Rc::new(RefCell::new(move |_, _| result.clone())) }
    }
}
impl<Arg0, Arg1, Res: 'static> CallMatch2<Arg0, Arg1, Res> {
    pub fn and_call_clone<F>(self, func: F) -> Reaction2<Arg0, Arg1, Res>
            where F: FnMut(Arg0, Arg1) -> Res + 'static {
        Reaction2 { call_match: self, action: Rc::new(RefCell::new(func)) }
    }
}
impl<Arg0, Arg1, Res: Default + 'static> CallMatch2<Arg0, Arg1, Res> {
    pub fn and_return_default(self) -> Reaction2<Arg0, Arg1, Res> {
        Reaction2 { call_match: self, action: Rc::new(RefCell::new(|_, _| Res::default())) }
    }
}


#[must_use]
pub struct CallMatch3<Arg0, Arg1, Arg2, Res> {
    mock_id: usize,
    mock_type_id: usize,
    method_name: &'static str,
    arg0: Box<dyn MatchArg<Arg0>>,
    arg1: Box<dyn MatchArg<Arg1>>,
    arg2: Box<dyn MatchArg<Arg2>>,

    _phantom: PhantomData<Res>,
}
impl<Arg0, Arg1, Arg2, Res> CallMatch3<Arg0, Arg1, Arg2, Res> {
    pub fn new(mock_id: usize, mock_type_id: usize, method_name: &'static str,
               arg0: Box<dyn MatchArg<Arg0>>,
               arg1: Box<dyn MatchArg<Arg1>>,
               arg2: Box<dyn MatchArg<Arg2>>) -> Self {
        CallMatch3 {
            mock_id: mock_id,
            mock_type_id: mock_type_id,
            method_name: method_name,
            arg0: arg0,
            arg1: arg1,
            arg2: arg2,
            _phantom: PhantomData
        }
    }

    fn get_args_ref(call: &Call) -> &(Arg0, Arg1, Arg2) {
        unsafe { &mut *(call.args_ptr as *mut (Arg0, Arg1, Arg2)) }
    }

    fn get_args(mut call: Call) -> Box<(Arg0, Arg1, Arg2)> {
        unsafe { Box::from_raw(call.take_args() as *mut (Arg0, Arg1, Arg2)) }
    }
}
impl<Arg0, Arg1, Arg2, Res> CallMatch for CallMatch3<Arg0, Arg1, Arg2, Res> {
    fn matches_args(&self, call: &Call) -> bool {
        assert!(call.method_data.mock_type_id == self.mock_type_id &&
                call.method_data.method_name == self.method_name);

        let args = Self::get_args_ref(call);
        self.arg0.matches(&args.0).is_ok() &&
        self.arg1.matches(&args.1).is_ok() &&
        self.arg2.matches(&args.2).is_ok()
    }
    fn validate(&self, call: &Call) -> Vec<Result<(), String>> {
        let args = Self::get_args_ref(call);
        vec![ self.arg0.matches(&args.0),
              self.arg1.matches(&args.1),
              self.arg2.matches(&args.2) ]
    }
    fn get_mock_id(&self) -> usize { self.mock_id }
    fn get_mock_type_id(&self) -> usize { self.mock_type_id }
    fn get_method_name(&self) -> &'static str { self.method_name }
    fn describe(&self) -> String {
        format!("{}({}, {}, {})", self.get_method_name(),
                                  self.arg0.describe(),
                                  self.arg1.describe(),
                                  self.arg2.describe())
    }
}

#[must_use]
pub struct Reaction3<Arg0, Arg1, Arg2, Res> {
    call_match: CallMatch3<Arg0, Arg1, Arg2, Res>,
    action: ActionClone3<Arg0, Arg1, Arg2, Res>,
}
impl<Arg0, Arg1, Arg2, Res> Reaction3<Arg0, Arg1, Arg2, Res> {
    pub fn times<C: Cardinality + 'static>(self, cardinality: C)
        -> ExpectationTimes3<Arg0, Arg1, Arg2, Res> {
        ExpectationTimes3::new(self.call_match, self.action, Box::new(cardinality))
    }
}

#[must_use]
pub struct ExpectationTimes3<Arg0, Arg1, Arg2, Res> {
    action: ActionClone3<Arg0, Arg1, Arg2, Res>,
    call_match: CallMatch3<Arg0, Arg1, Arg2, Res>,
    cardinality: Box<dyn Cardinality>,
    count: u32,
}
impl<Arg0, Arg1, Arg2, Res> ExpectationTimes3<Arg0, Arg1, Arg2, Res> {
    fn new(call_match: CallMatch3<Arg0, Arg1, Arg2, Res>, action: ActionClone3<Arg0, Arg1, Arg2, Res>, cardinality: Box<dyn Cardinality>) -> Self {
        ExpectationTimes3 { call_match: call_match, action: action, cardinality: cardinality, count: 0 }
    }
}
impl<Arg0: 'static, Arg1: 'static, Arg2: 'static, Res: 'static> Expectation for ExpectationTimes3<Arg0, Arg1, Arg2, Res> {
    fn call_match(&self) -> &dyn CallMatch {
        &self.call_match
    }
    fn is_satisfied(&self) -> bool {
        self.cardinality.check(self.count) == CardinalityCheckResult::Satisfied
    }
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        self.count += 1;
        if self.cardinality.check(self.count) == CardinalityCheckResult::Wrong {
            panic!("{}.{} is called for the {} time, but expected to be {}",
                   mock_name, self.call_match().get_method_name(), format_ordinal(self.count),
                   self.cardinality.describe_upper_bound());
        }
        // nightly: let box (arg0, arg1, arg2) = CallMatch3::<Arg0, Arg1, Arg2, Res>::get_args(call);
        let (arg0, arg1, arg2) = *CallMatch3::<Arg0, Arg1, Arg2, Res>::get_args(call);
        box_fn::BoxFn0::new({
            let action = self.action.clone();
            move || {
                let result = action.borrow_mut().deref_mut()(arg0, arg1, arg2);
                Box::into_raw(Box::new(result)) as *mut u8
            }
        })
    }
    fn describe(&self) -> String {
        format!("{} must be {}, called {} times",
                self.call_match.describe(), self.cardinality.describe(), self.count)
    }
}

#[must_use]
pub struct Expectation3<Arg0, Arg1, Arg2, Res> {
    call_match: CallMatch3<Arg0, Arg1, Arg2, Res>,
    action: Option<Action3<Arg0, Arg1, Arg2, Res>>,
}
impl<Arg0: 'static, Arg1: 'static, Arg2: 'static, Res: 'static> Expectation for Expectation3<Arg0, Arg1, Arg2, Res> {
    fn call_match(&self) -> &dyn CallMatch {
        &self.call_match
    }
    fn is_satisfied(&self) -> bool {
        self.action.is_none()
    }
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        match self.action.take() {
            Some(action) => {
                // nightly: let box (arg0, arg1, arg2) = CallMatch3::<Arg0, Arg1, Arg2, Res>::get_args(call);
                let (arg0, arg1, arg2) = *CallMatch3::<Arg0, Arg1, Arg2, Res>::get_args(call);
                box_fn::BoxFn0::new(move || {
                    let result = action.call(arg0, arg1, arg2);
                    Box::into_raw(Box::new(result)) as *mut u8
                })
            },
            None => {
                panic!("{}.{} was already called earlier", mock_name, self.call_match().get_method_name());
            }
        }
    }
    fn describe(&self) -> String {
        self.call_match.describe()
    }
}
impl<Arg0, Arg1, Arg2, Res: 'static> CallMatch3<Arg0, Arg1, Arg2, Res> {
    pub fn and_return(self, result: Res) -> Expectation3<Arg0, Arg1, Arg2, Res> {
        Expectation3 { call_match: self, action: Some(Action3::new(move |_, _, _| result)) }
    }

    pub fn and_panic(self, msg: String) -> Expectation3<Arg0, Arg1, Arg2, Res> {
        Expectation3 { call_match: self, action: Some(Action3::new(move |_, _, _| panic!(msg))) }
    }

    pub fn and_call<F>(self, func: F) -> Expectation3<Arg0, Arg1, Arg2, Res>
            where F: FnOnce(Arg0, Arg1, Arg2) -> Res + 'static {
        Expectation3 { call_match: self, action: Some(Action3::new(func)) }
    }

    pub fn never(self) -> ExpectationNever<Self> {
        ExpectationNever { call_match: self }
    }
}
impl<Arg0, Arg1, Arg2, Res: Clone + 'static> CallMatch3<Arg0, Arg1, Arg2, Res> {
    pub fn and_return_clone(self, result: Res) -> Reaction3<Arg0, Arg1, Arg2, Res> {
        Reaction3 { call_match: self, action: Rc::new(RefCell::new(move |_, _, _| result.clone())) }
    }
}
impl<Arg0, Arg1, Arg2, Res: 'static> CallMatch3<Arg0, Arg1, Arg2, Res> {
    pub fn and_call_clone<F>(self, func: F) -> Reaction3<Arg0, Arg1, Arg2, Res>
            where F: FnMut(Arg0, Arg1, Arg2) -> Res + 'static {
        Reaction3 { call_match: self, action: Rc::new(RefCell::new(func)) }
    }
}
impl<Arg0, Arg1, Arg2, Res: Default + 'static> CallMatch3<Arg0, Arg1, Arg2, Res> {
    pub fn and_return_default(self) -> Reaction3<Arg0, Arg1, Arg2, Res> {
        Reaction3 { call_match: self, action: Rc::new(RefCell::new(move |_, _, _| Res::default())) }
    }
}

#[must_use]
pub struct CallMatch4<Arg0, Arg1, Arg2, Arg3, Res> {
    mock_id: usize,
    mock_type_id: usize,
    method_name: &'static str,
    arg0: Box<dyn MatchArg<Arg0>>,
    arg1: Box<dyn MatchArg<Arg1>>,
    arg2: Box<dyn MatchArg<Arg2>>,
    arg3: Box<dyn MatchArg<Arg3>>,

    _phantom: PhantomData<Res>,
}
impl<Arg0, Arg1, Arg2, Arg3, Res> CallMatch4<Arg0, Arg1, Arg2, Arg3, Res> {
    pub fn new(mock_id: usize, mock_type_id: usize, method_name: &'static str,
               arg0: Box<dyn MatchArg<Arg0>>,
               arg1: Box<dyn MatchArg<Arg1>>,
               arg2: Box<dyn MatchArg<Arg2>>,
               arg3: Box<dyn MatchArg<Arg3>>) -> Self {
        CallMatch4 {
            mock_id: mock_id,
            mock_type_id: mock_type_id,
            method_name: method_name,
            arg0: arg0,
            arg1: arg1,
            arg2: arg2,
            arg3: arg3,
            _phantom: PhantomData
        }
    }

    fn get_args_ref(call: &Call) -> &(Arg0, Arg1, Arg2, Arg3) {
        unsafe { &mut *(call.args_ptr as *mut (Arg0, Arg1, Arg2, Arg3)) }
    }

    fn get_args(mut call: Call) -> Box<(Arg0, Arg1, Arg2, Arg3)> {
        unsafe { Box::from_raw(call.take_args() as *mut (Arg0, Arg1, Arg2, Arg3)) }
    }
}
impl<Arg0, Arg1, Arg2, Arg3, Res> CallMatch for CallMatch4<Arg0, Arg1, Arg2, Arg3, Res> {
    fn matches_args(&self, call: &Call) -> bool {
        assert!(call.method_data.mock_type_id == self.mock_type_id &&
                call.method_data.method_name == self.method_name);

        let args = Self::get_args_ref(call);
        self.arg0.matches(&args.0).is_ok() &&
        self.arg1.matches(&args.1).is_ok() &&
        self.arg2.matches(&args.2).is_ok() &&
        self.arg3.matches(&args.3).is_ok()
    }
    fn validate(&self, call: &Call) -> Vec<Result<(), String>> {
        let args = Self::get_args_ref(call);
        vec![ self.arg0.matches(&args.0),
              self.arg1.matches(&args.1),
              self.arg2.matches(&args.2),
              self.arg3.matches(&args.3) ]
    }
    fn get_mock_id(&self) -> usize { self.mock_id }
    fn get_mock_type_id(&self) -> usize { self.mock_type_id }
    fn get_method_name(&self) -> &'static str { self.method_name }
    fn describe(&self) -> String {
        format!("{}({}, {}, {}, {})", self.get_method_name(),
                                      self.arg0.describe(),
                                      self.arg1.describe(),
                                      self.arg2.describe(),
                                      self.arg3.describe())
    }
}

#[must_use]
pub struct Reaction4<Arg0, Arg1, Arg2, Arg3, Res> {
    call_match: CallMatch4<Arg0, Arg1, Arg2, Arg3, Res>,
    action: ActionClone4<Arg0, Arg1, Arg2, Arg3, Res>,
}
impl<Arg0, Arg1, Arg2, Arg3, Res> Reaction4<Arg0, Arg1, Arg2, Arg3, Res> {
    pub fn times<C: Cardinality + 'static>(self, cardinality: C)
        -> ExpectationTimes4<Arg0, Arg1, Arg2, Arg3, Res> {
        ExpectationTimes4::new(self.call_match, self.action, Box::new(cardinality))
    }
}

#[must_use]
pub struct ExpectationTimes4<Arg0, Arg1, Arg2, Arg3, Res> {
    action: ActionClone4<Arg0, Arg1, Arg2, Arg3, Res>,
    call_match: CallMatch4<Arg0, Arg1, Arg2, Arg3, Res>,
    cardinality: Box<dyn Cardinality>,
    count: u32,
}
impl<Arg0, Arg1, Arg2, Arg3, Res> ExpectationTimes4<Arg0, Arg1, Arg2, Arg3, Res> {
    fn new(call_match: CallMatch4<Arg0, Arg1, Arg2, Arg3, Res>, action: ActionClone4<Arg0, Arg1, Arg2, Arg3, Res>, cardinality: Box<dyn Cardinality>) -> Self {
        ExpectationTimes4 { call_match: call_match, action: action, cardinality: cardinality, count: 0 }
    }
}
impl<Arg0: 'static, Arg1: 'static, Arg2: 'static, Arg3: 'static, Res: 'static> Expectation for ExpectationTimes4<Arg0, Arg1, Arg2, Arg3, Res> {
    fn call_match(&self) -> &dyn CallMatch {
        &self.call_match
    }
    fn is_satisfied(&self) -> bool {
        self.cardinality.check(self.count) == CardinalityCheckResult::Satisfied
    }
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        self.count += 1;
        if self.cardinality.check(self.count) == CardinalityCheckResult::Wrong {
            panic!("{}.{} is called for the {} time, but expected to be {}",
                   mock_name, self.call_match().get_method_name(), format_ordinal(self.count),
                   self.cardinality.describe_upper_bound());
        }
        // nightly: let box (arg0, arg1, arg2, arg3) = CallMatch4::<Arg0, Arg1, Arg2, Arg3, Res>::get_args(call);
        let (arg0, arg1, arg2, arg3) = *CallMatch4::<Arg0, Arg1, Arg2, Arg3, Res>::get_args(call);
        box_fn::BoxFn0::new({
            let action = self.action.clone();
            move || {
                let result = action.borrow_mut().deref_mut()(arg0, arg1, arg2, arg3);
                Box::into_raw(Box::new(result)) as *mut u8
            }
        })
    }
    fn describe(&self) -> String {
        format!("{} must be {}, called {} times",
                self.call_match.describe(), self.cardinality.describe(), self.count)
    }
}

#[must_use]
pub struct Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
    call_match: CallMatch4<Arg0, Arg1, Arg2, Arg3, Res>,
    action: Option<Action4<Arg0, Arg1, Arg2, Arg3, Res>>,
}
impl<Arg0: 'static, Arg1: 'static, Arg2: 'static, Arg3: 'static, Res: 'static> Expectation for Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
    fn call_match(&self) -> &dyn CallMatch {
        &self.call_match
    }
    fn is_satisfied(&self) -> bool {
        self.action.is_none()
    }
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        match self.action.take() {
            Some(action) => {
                // nightly: let box (arg0, arg1, arg2, arg3) = CallMatch4::<Arg0, Arg1, Arg2, Arg3, Res>::get_args(call);
                let (arg0, arg1, arg2, arg3) = *CallMatch4::<Arg0, Arg1, Arg2, Arg3, Res>::get_args(call);
                box_fn::BoxFn0::new(move || {
                    let result = action.call(arg0, arg1, arg2, arg3);
                    Box::into_raw(Box::new(result)) as *mut u8
                })
            },
            None => {
                panic!("{}.{} was already called earlier", mock_name, self.call_match().get_method_name());
            }
        }
    }
    fn describe(&self) -> String {
        self.call_match.describe()
    }
}
impl<Arg0, Arg1, Arg2, Arg3, Res: 'static> CallMatch4<Arg0, Arg1, Arg2, Arg3, Res> {
    pub fn and_return(self, result: Res) -> Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
        Expectation4 { call_match: self, action: Some(Action4::new(move |_, _, _, _| result)) }
    }

    pub fn and_panic(self, msg: String) -> Expectation4<Arg0, Arg1, Arg2, Arg3, Res> {
        Expectation4 { call_match: self, action: Some(Action4::new(move |_, _, _, _| panic!(msg))) }
    }

    pub fn and_call<F>(self, func: F) -> Expectation4<Arg0, Arg1, Arg2, Arg3, Res>
            where F: FnOnce(Arg0, Arg1, Arg2, Arg3) -> Res + 'static {
        Expectation4 { call_match: self, action: Some(Action4::new(func)) }
    }

    pub fn never(self) -> ExpectationNever<Self> {
        ExpectationNever { call_match: self }
    }
}
impl<Arg0, Arg1, Arg2, Arg3, Res: Clone + 'static> CallMatch4<Arg0, Arg1, Arg2, Arg3, Res> {
    pub fn and_return_clone(self, result: Res) -> Reaction4<Arg0, Arg1, Arg2, Arg3, Res> {
        Reaction4 { call_match: self, action: Rc::new(RefCell::new(move |_, _, _, _| result.clone())) }
    }
}
impl<Arg0, Arg1, Arg2, Arg3, Res: 'static> CallMatch4<Arg0, Arg1, Arg2, Arg3, Res> {
    pub fn and_call_clone<F>(self, func: F) -> Reaction4<Arg0, Arg1, Arg2, Arg3, Res>
            where F: FnMut(Arg0, Arg1, Arg2, Arg3) -> Res + 'static {
        Reaction4 { call_match: self, action: Rc::new(RefCell::new(func)) }
    }
}
impl<Arg0, Arg1, Arg2, Arg3, Res: Default + 'static> CallMatch4<Arg0, Arg1, Arg2, Arg3, Res> {
    pub fn and_return_default(self) -> Reaction4<Arg0, Arg1, Arg2, Arg3, Res> {
        Reaction4 { call_match: self, action: Rc::new(RefCell::new(|_, _, _, _| Res::default())) }
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

#[derive(Default)]
pub struct Sequence {
    expectations: Vec<Box<dyn Expectation>>,
}
impl Sequence {
    pub fn new() -> Self {
        Sequence { expectations: Vec::new() }
    }

    pub fn expect<E: Expectation + 'static>(&mut self, expectation: E) {
        assert!(!expectation.is_satisfied());
        self.expectations.push(Box::new(expectation));
    }
}
impl Expectation for Sequence {
    fn call_match(&self) -> &dyn CallMatch {
        self.expectations[0].call_match()
    }
    fn is_satisfied(&self) -> bool {
        self.expectations.is_empty()
    }
    fn satisfy(&mut self, call: Call, mock_name: &str) -> box_fn::BoxFn0<*mut u8> {
        let (res, remove) = {
            let exp = &mut self.expectations[0];
            let res = exp.satisfy(call, mock_name);
            (res, exp.is_satisfied())
        };

        if remove {
            self.expectations.remove(0);
        }

        res
    }
    fn describe(&self) -> String {
        self.expectations[0].describe()
    }
}

pub trait Mock {
    fn new(id: usize, scenario_int: Rc<RefCell<ScenarioInternals>>) -> Self;
    fn mocked_class_name() -> &'static str;
}

pub trait Mocked {
    type MockImpl: Mock;
}

pub struct ScenarioInternals {
    expectations: Vec<Box<dyn Expectation>>,

    next_mock_id: usize,

    /// Mapping from mock ID to mock name.
    mock_names: HashMap<usize, Rc<String>>,
    /// Set of used mock names used to quicly check for conflicts.
    allocated_names: HashSet<Rc<String>>,
}

impl ScenarioInternals {
    fn get_next_mock_id(&mut self) -> usize {
        let id = self.next_mock_id;
        self.next_mock_id += 1;
        id
    }

    pub fn create_mock<T: Mock>(int: &Rc<RefCell<Self>>) -> T {
        let mut internals = int.borrow_mut();
        let mock_id = internals.get_next_mock_id();
        internals.generate_name_for_class(mock_id, T::mocked_class_name());
        T::new(mock_id, int.clone())
    }

    pub fn create_mock_with_id<T: Mock>(int: &Rc<RefCell<Self>>, mock_id: usize) -> T {
        T::new(mock_id, int.clone())
    }

    pub fn create_named_mock<T: Mock>(int: &Rc<RefCell<Self>>, name: String) -> T {
        let mut internals = int.borrow_mut();
        let mock_id = internals.get_next_mock_id();
        internals.register_name(mock_id, name);
        T::new(mock_id, int.clone())
    }

    pub fn create_mock_for<T: ?Sized>(int: &Rc<RefCell<Self>>) -> <&'static T as Mocked>::MockImpl
            where &'static T: Mocked {
        Self::create_mock::<<&'static T as Mocked>::MockImpl>(int)
    }

    pub fn create_named_mock_for<T: ?Sized>(int: &Rc<RefCell<Self>>, name: String) -> <&'static T as Mocked>::MockImpl
            where &'static T: Mocked {
        Self::create_named_mock::<<&'static T as Mocked>::MockImpl>(int, name)
    }

    pub fn generate_name_for_class(&mut self, mock_id: usize, class_name: &str) {
        for i in 0.. {
            let name = format!("{}#{}", class_name, i);
            if !self.allocated_names.contains(&name) {
                let name_rc = Rc::new(name);
                self.mock_names.insert(mock_id, name_rc.clone());
                self.allocated_names.insert(name_rc);
                break;
            }
        }
    }

    fn register_name(&mut self, mock_id: usize, name: String) {
        if self.allocated_names.contains(&name) {
            panic!("Mock name {} already used", name);
        }
        let name_rc = Rc::new(name);
        self.mock_names.insert(mock_id, name_rc.clone());
        self.allocated_names.insert(name_rc);
    }
}

pub struct Scenario {
    internals: Rc<RefCell<ScenarioInternals>>,
}

impl Scenario {
    pub fn new() -> Self {
        Scenario {
            internals: Rc::new(RefCell::new(ScenarioInternals {
                expectations: Vec::new(),
                next_mock_id: 0,

                mock_names: HashMap::new(),
                allocated_names: HashSet::new(),
            })),
        }
    }

    pub fn create_mock<T: Mock>(&self) -> T {
        ScenarioInternals::create_mock::<T>(&self.internals)
    }

    pub fn create_named_mock<T: Mock>(&self, name: String) -> T {
        ScenarioInternals::create_named_mock::<T>(&self.internals, name)
    }

    pub fn create_mock_for<T: ?Sized>(&self) -> <&'static T as Mocked>::MockImpl
            where &'static T: Mocked {
        ScenarioInternals::create_mock_for::<T>(&self.internals)
    }

    pub fn create_named_mock_for<T: ?Sized>(&self, name: String) -> <&'static T as Mocked>::MockImpl
            where &'static T: Mocked {
        ScenarioInternals::create_named_mock_for::<T>(&self.internals, name)
    }

    pub fn expect<C: Expectation + 'static>(&self, call: C) {
        self.internals.borrow_mut().expectations.push(Box::new(call));
    }

    pub fn checkpoint(&self) {
        self.verify_expectations();
        self.internals.borrow_mut().expectations.clear();
    }

    pub fn handle(&self) -> ScenarioHandle {
      ScenarioHandle::new(Rc::downgrade(&self.internals))
    }

    fn verify_expectations(&self) {
        let int = self.internals.borrow();
        let expectations = &int.expectations;
        let mock_names = &int.mock_names;
        let mut active_expectations = expectations.iter().filter(|e| !e.is_satisfied()).peekable();
        if active_expectations.peek().is_some() {
            let mut s = String::from("Some expectations are not satisfied:\n");
            for expectation in active_expectations {
                let mock_name = mock_names.get(&expectation.call_match().get_mock_id()).unwrap();
                s.push_str(&format!("`{}.{}`\n", mock_name, expectation.describe()));
            }
            panic!(s);
        }
    }
}

impl Default for Scenario {
    fn default() -> Self { Self::new() }
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

        self.verify_expectations();
    }
}

pub struct ScenarioHandle {
    internals: Weak<RefCell<ScenarioInternals>>,
}

impl ScenarioHandle {
    pub fn new(scenario_int: Weak<RefCell<ScenarioInternals>>) -> Self {
        Self { internals: scenario_int }
    }

    pub fn create_mock<T: Mock>(&self) -> T {
        ScenarioInternals::create_mock::<T>(&self.get_internals())
    }

    pub fn create_named_mock<T: Mock>(&self, name: String) -> T {
        ScenarioInternals::create_named_mock::<T>(&self.get_internals(), name)
    }

    pub fn create_mock_for<T: ?Sized>(&self) -> <&'static T as Mocked>::MockImpl
            where &'static T: Mocked {
        ScenarioInternals::create_mock_for::<T>(&self.get_internals())
    }

    pub fn create_named_mock_for<T: ?Sized>(&self, name: String) -> <&'static T as Mocked>::MockImpl
            where &'static T: Mocked {
        ScenarioInternals::create_named_mock_for::<T>(&self.get_internals(), name)
    }

    pub fn expect<C: Expectation + 'static>(&self, call: C) {
        let ints = self.get_internals();
        ints.borrow_mut().expectations.push(Box::new(call));
    }

    fn get_internals(&self) -> Rc<RefCell<ScenarioInternals>> {
      self.internals.upgrade().expect("scenario is dead")
    }
}

pub struct Call {
    pub method_data: MethodData,
    pub args_ptr: *const u8,
    pub destroy: fn(*const u8),
    pub format_args: fn(*const u8) -> String,
}
impl Call {
    pub fn take_args(&mut self) -> *const u8 {
        std::mem::replace(&mut self.args_ptr, std::ptr::null())
    }
}
impl Drop for Call {
    fn drop(&mut self) {
        if !self.args_ptr.is_null() {
            (self.destroy)(self.args_ptr);
        }
    }
}

// Copied from 'colorify' package because it has no plain 'bold' variant.
macro_rules! colored {
    (bold: $s:expr) => ( concat!("\x1b[1m", $s, "\x1b[0m") );
    (red: $s:expr) => ( concat!("\x1b[31m", $s, "\x1b[0m") );
    (red_bold: $s:expr) => ( concat!("\x1b[1;31m", $s, "\x1b[0m") );
    (green: $s:expr) => ( concat!("\x1b[32m", $s, "\x1b[0m") );
    (green_bold: $s:expr) => ( concat!("\x1b[1;32m", $s, "\x1b[0m") );
    (orange: $s:expr) => ( concat!("\x1b[33m", $s, "\x1b[0m") );
    (yellow_bold: $s:expr) => ( concat!("\x1b[1;33m", $s, "\x1b[0m") );
    (blue: $s:expr) => ( concat!("\x1b[34m", $s, "\x1b[0m") );
    (blue_bold: $s:expr) => ( concat!("\x1b[1;34m", $s, "\x1b[0m") );
    (purple: $s:expr) => ( concat!("\x1b[35m", $s, "\x1b[0m") );
    (purple_bold: $s:expr) => ( concat!("\x1b[1;35m", $s, "\x1b[0m") );
    (cyan: $s:expr) => ( concat!("\x1b[36m", $s, "\x1b[0m") );
    (cyan_bold: $s:expr) => ( concat!("\x1b[1;36m", $s, "\x1b[0m") );
    (light_grey: $s:expr) => ( concat!("\x1b[37m", $s, "\x1b[0m") );
    (white_bold: $s:expr) => ( concat!("\x1b[1;37m", $s, "\x1b[0m") );
    (dark_grey: $s:expr) => ( concat!("\x1b[90m", $s, "\x1b[0m") );
    (dark_grey_bold: $s:expr) => ( concat!("\x1b[1;90m", $s, "\x1b[0m") );
    (peach: $s:expr) => ( concat!("\x1b[91m", $s, "\x1b[0m") );
    (peach_bold: $s:expr) => ( concat!("\x1b[1;91m", $s, "\x1b[0m") );
    (lime: $s:expr) => ( concat!("\x1b[92m", $s, "\x1b[0m") );
    (lime_bold: $s:expr) => ( concat!("\x1b[1;92m", $s, "\x1b[0m") );
    (yellow: $s:expr) => ( concat!("\x1b[93m", $s, "\x1b[0m") );
    (yellow_bold: $s:expr) => ( concat!("\x1b[1;93m", $s, "\x1b[0m") );
    (royal_blue: $s:expr) => ( concat!("\x1b[94m", $s, "\x1b[0m") );
    (royal_blue_bold: $s:expr) => ( concat!("\x1b[1;94m", $s, "\x1b[0m") );
    (magenta: $s:expr) => ( concat!("\x1b[95m", $s, "\x1b[0m") );
    (magenta_bold: $s:expr) => ( concat!("\x1b[1;95m", $s, "\x1b[0m") );
    (teal: $s:expr) => ( concat!("\x1b[96m", $s, "\x1b[0m") );
    (teal_bold: $s:expr) => ( concat!("\x1b[1;96m", $s, "\x1b[0m") );
    (white: $s:expr) => ( concat!("\x1b[97m", $s, "\x1b[0m") );
    (white_bold: $s:expr) => ( concat!("\x1b[1;97m", $s, "\x1b[0m") );
}

pub struct MethodData {
    pub mock_id: usize,
    pub mock_type_id: usize,
    pub method_name: &'static str,
}

impl ScenarioInternals {
    pub fn verify0<Res>(&mut self, method_data: MethodData) -> box_fn::BoxFn0<Res> {
        let args = Box::new(());
        let args_ptr: *const u8 =
            ::std::boxed::Box::into_raw(args) as *const u8;
        fn destroy(args_to_destroy: *const u8) {
            unsafe { Box::from_raw(args_to_destroy as *mut ()) };
        };
        fn format_args(args_ptr: *const u8) -> String {
            let _args_ref: &() =
                unsafe { ::std::mem::transmute(args_ptr) };
            format!("")
        };
        let call = Call { method_data: method_data,
                          args_ptr: args_ptr,
                          destroy: destroy,
                          format_args: format_args, };
        let action = self.verify(call);
        box_fn::BoxFn0::new(move || {
            let result_ptr: *mut u8 = action.call();
            let result: Box<Res> =
                unsafe {
                    Box::from_raw(result_ptr as *mut Res)
                };
            *result
        })
    }

    pub fn verify1<A0, Res>(&mut self, method_data: MethodData, a0: A0) -> box_fn::BoxFn0<Res> {
        let args = Box::new((a0,));
        let args_ptr: *const u8 =
            ::std::boxed::Box::into_raw(args) as *const u8;
        fn destroy<A0>(args_to_destroy: *const u8) {
            unsafe { Box::from_raw(args_to_destroy as *mut (A0,)) };
        };
        fn format_args<A0>(args_ptr: *const u8) -> String {
            let _args_ref: &(A0,) =
                unsafe { ::std::mem::transmute(args_ptr) };
            format!("{:?}", dbg(&_args_ref.0))
        };
        let call = Call { method_data: method_data,
                          args_ptr: args_ptr,
                          destroy: destroy::<A0>,
                          format_args: format_args::<A0>, };
        let action = self.verify(call);
        box_fn::BoxFn0::new(move || {
            let result_ptr: *mut u8 = action.call();
            let result: Box<Res> =
                unsafe {
                    Box::from_raw(result_ptr as *mut Res)
                };
            *result
        })
    }

    pub fn verify2<A0, A1, Res>(
            &mut self, method_data: MethodData, a0: A0, a1: A1) -> box_fn::BoxFn0<Res> {
        let args = Box::new((a0, a1));
        let args_ptr: *const u8 =
            ::std::boxed::Box::into_raw(args) as *const u8;
        fn destroy<A0, A1>(args_to_destroy: *const u8) {
            unsafe { Box::from_raw(args_to_destroy as *mut (A0, A1)) };
        };
        fn format_args<A0, A1>(args_ptr: *const u8) -> String {
            let _args_ref: &(A0, A1) =
                unsafe { ::std::mem::transmute(args_ptr) };
            format!("{:?}, {:?}", dbg(&_args_ref.0), dbg(&_args_ref.1))
        };
        let call = Call { method_data: method_data,
                          args_ptr: args_ptr,
                          destroy: destroy::<A0, A1>,
                          format_args: format_args::<A0, A1>, };
        let action = self.verify(call);
        box_fn::BoxFn0::new(move || {
            let result_ptr: *mut u8 = action.call();
            let result: Box<Res> =
                unsafe {
                    Box::from_raw(result_ptr as *mut Res)
                };
            *result
        })
    }

    pub fn verify3<A0, A1, A2, Res>
            (&mut self, method_data: MethodData, a0: A0, a1: A1, a2: A2) -> box_fn::BoxFn0<Res> {
        let args = Box::new((a0, a1, a2));
        let args_ptr: *const u8 =
            ::std::boxed::Box::into_raw(args) as *const u8;
        fn destroy<A0, A1, A2>(args_to_destroy: *const u8) {
            unsafe { Box::from_raw(args_to_destroy as *mut (A0, A1, A2)) };
        };
        fn format_args<A0, A1, A2>
                (args_ptr: *const u8) -> String {
            let _args_ref: &(A0, A1, A2) =
                unsafe { ::std::mem::transmute(args_ptr) };
            format!("{:?}, {:?}, {:?}", dbg(&_args_ref.0), dbg(&_args_ref.1), dbg(&_args_ref.2))
        };
        let call = Call { method_data: method_data,
                          args_ptr: args_ptr,
                          destroy: destroy::<A0, A1, A2>,
                          format_args: format_args::<A0, A1, A2>, };
        let action = self.verify(call);
        box_fn::BoxFn0::new(move || {
            let result_ptr: *mut u8 = action.call();
            let result: Box<Res> =
                unsafe {
                    Box::from_raw(result_ptr as *mut Res)
                };
            *result
        })
    }

    pub fn verify4<A0, A1, A2, A3, Res>
            (&mut self, method_data: MethodData, a0: A0, a1: A1, a2: A2, a3: A3) -> box_fn::BoxFn0<Res> {
        let args = Box::new((a0, a1, a2, a3));
        let args_ptr: *const u8 =
            ::std::boxed::Box::into_raw(args) as *const u8;
        fn destroy<A0, A1, A2, A3>(args_to_destroy: *const u8) {
            unsafe { Box::from_raw(args_to_destroy as *mut (A0, A1, A2, A3)) };
        };
        fn format_args<A0, A1, A2, A3>
                (args_ptr: *const u8) -> String {
            let args_ref: &(A0, A1, A2, A3) =
                unsafe { ::std::mem::transmute(args_ptr) };
            format!("{:?}, {:?}, {:?}, {:?}", dbg(&args_ref.0), dbg(&args_ref.1), dbg(&args_ref.2), dbg(&args_ref.3))
        };
        let call = Call { method_data: method_data,
                          args_ptr: args_ptr,
                          destroy: destroy::<A0, A1, A2, A3>,
                          format_args: format_args::<A0, A1, A2, A3>, };
        let action = self.verify(call);
        box_fn::BoxFn0::new(move || {
            let result_ptr: *mut u8 = action.call();
            let result: Box<Res> =
                unsafe {
                    Box::from_raw(result_ptr as *mut Res)
                };
            *result
        })
    }

    /// Verify call performed on mock object
    /// Returns closure which returns result upon call.
    /// Closure returned instead of actual result, because expectation may
    /// use user-provided closure as action, and that closure may want to
    /// use scenario object to create mocks or establish expectations, so
    /// we need to release scenario borrow before calling expectation action.
    fn verify(&mut self, call: Call) -> box_fn::BoxFn0<*mut u8> {

        for expectation in self.expectations.iter_mut().rev() {
            if expectation.call_match().matches(&call) {
                let mock_name = self.mock_names.get(&call.method_data.mock_id).unwrap().clone();
                return expectation.satisfy(call, &mock_name);
            }
        }

        // No expectations exactly matching call are found. However this may be
        // because of unexpected argument values. So check active expectations
        // with matching target (i.e. mock and method) and validate arguments.
        let mock_name = self.mock_names.get(&call.method_data.mock_id).unwrap();

        let mut msg = String::new();
        msg.write_str("\n\n").unwrap();
        write!(&mut msg,
               concat!(colored!(red: "error:"), " ",
                       colored!(bold: "unexpected call to `{}.{}({})`\n\n")),
               mock_name, call.method_data.method_name, (call.format_args)(call.args_ptr)).unwrap();

        if self.expectations.is_empty() {
            msg.push_str("no call are expected");
            panic!(msg);
        }

        let mut target_first_match = true;
        for expectation in self.expectations.iter().rev() {
            if !expectation.is_satisfied() && expectation.call_match().matches_target(&call) {
                if target_first_match {
                    write!(&mut msg, concat!(colored!(green: "note: "),
                                            "here are active expectations for {}.{}\n"),
                                     mock_name, call.method_data.method_name).unwrap();
                    target_first_match = false;
                }

                write!(&mut msg, "\n  expectation `{}.{}`:\n", mock_name, expectation.describe()).unwrap();
                for (index, res) in expectation.call_match().validate(&call).iter().enumerate() {
                    match *res {
                        Err(ref err) =>
                            write!(&mut msg, concat!("    arg #{}: ", colored!(bold: "{}"), "\n"),
                                   index, err).unwrap(),
                        Ok(()) => ()
                    }
                }
            }
        }

        if target_first_match {
            write!(&mut msg, concat!(colored!(green: "note: "), "there are no active expectations for {}.{}\n"),
                   mock_name, call.method_data.method_name).unwrap();
        }

        let mut method_first_match = true;
        for expectation in self.expectations.iter().rev() {
            if !expectation.is_satisfied() &&
               !expectation.call_match().matches_target(&call) &&
               expectation.call_match().matches_method(&call) &&
               expectation.call_match().matches_args(&call) {

                if method_first_match {
                    msg.push_str(concat!(colored!(green: "note: "),
                                        "there are matching expectations for another mock objects\n"));
                    method_first_match = false;
                }

                let other_mock_id = &expectation.call_match().get_mock_id();
                let other_mock_name = self.mock_names.get(other_mock_id).unwrap();
                write!(&mut msg, concat!("\n  expectation `", colored!(bold: "{}"), ".{}`\n"),
                       other_mock_name, expectation.describe()).unwrap();
            }
        }

        msg.push('\n');
        panic!(msg);
    }

    pub fn get_mock_name(&self, mock_id: usize) -> &str {
        self.mock_names.get(&mock_id).unwrap()
    }
}

fn format_ordinal(n: u32) -> String {
    match n % 10 {
        1 => format!("{}st", n),
        2 => format!("{}nd", n),
        3 => format!("{}rd", n),
        _ => format!("{}th", n),
    }
}
