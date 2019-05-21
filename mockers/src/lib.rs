#![cfg_attr(feature = "nightly", feature(specialization))]

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

use std::fmt::Write;
use std::ops::DerefMut;

use itertools::Itertools;

#[macro_use]
mod colors;
pub mod cardinality;
mod dbg;
pub mod matchers;
#[macro_use]
pub mod clone;
pub mod type_info;

pub use crate::type_info::TypeInfo;
pub use dbg::DebugOnStable;

use crate::cardinality::{Cardinality, CardinalityCheckResult};
use crate::dbg::dbg;

thread_local! {
    // Mapping from mock_type_id of 'extern' block mock to corresponding mock object.
    // It is needed since mock is object but mocked functions are static.
    pub static EXTERN_MOCKS: RefCell<HashMap<usize, (usize, Rc<RefCell<ScenarioInternals>>)>> = RefCell::new(HashMap::new());
}

macro_rules! define_actions {
    ($action_clone:ident { $($Arg:ident),* }) => {
        type $action_clone<$($Arg,)* T> = Rc<RefCell<dyn FnMut($($Arg,)*) -> T>>;
    }
}

define_actions!(ActionClone0 { });
define_actions!(ActionClone1 { Arg0 });
define_actions!(ActionClone2 { Arg0, Arg1 });
define_actions!(ActionClone3 { Arg0, Arg1, Arg2 });
define_actions!(ActionClone4 { Arg0, Arg1, Arg2, Arg3 });

pub trait CallMatch {
    fn matches_args(&self, call: &Call) -> bool;
    fn matches(&self, call: &Call) -> bool {
        self.matches_target(call) && self.matches_method(call) && self.matches_args(call)
    }
    fn matches_target(&self, call: &Call) -> bool {
        self.get_mock_id() == call.method_data.mock_id
    }
    fn matches_generic_method(&self, call: &Call) -> bool {
        self.get_mock_type_id() == call.method_data.mock_type_id
            && self.get_method_name() == call.method_data.method_name
    }
    fn matches_method(&self, call: &Call) -> bool {
        self.get_mock_type_id() == call.method_data.mock_type_id
            && self.get_method_name() == call.method_data.method_name
            && self.get_type_param_ids() == &call.method_data.type_param_ids[..]
    }
    fn validate(&self, call: &Call) -> Vec<Result<(), String>>;
    fn get_mock_id(&self) -> usize;
    fn get_mock_type_id(&self) -> usize;
    fn get_method_name(&self) -> &'static str;
    fn get_type_param_ids(&self) -> &[usize];
    fn describe(&self) -> String;
}

pub trait Expectation {
    fn call_match(&self) -> &dyn CallMatch;
    fn is_satisfied(&self) -> bool;
    fn satisfy(&mut self, call: Call, mock_name: &str) -> Box<Satisfy>;
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
    fn satisfy(&mut self, _call: Call, mock_name: &str) -> Box<Satisfy> {
        panic!(
            "{}.{} should never be called",
            mock_name,
            self.call_match().get_method_name()
        );
    }
    fn describe(&self) -> String {
        format!("{} should never be called", self.call_match.describe())
    }
}

pub trait Satisfy {
    fn satisfy(self: Box<Self>) -> *mut u8;
}

macro_rules! define_all {
    (
        ($call_match:ident, $reaction:ident,
          $action_clone:ident,
          $satisfy:ident, $satisfy_clone:ident,
          $expectation:ident, $expectation_times:ident)
        { $(($n:tt, $arg:ident, $Arg:ident)),* }
    ) => {
        struct $satisfy<$($Arg,)* Res, F: FnOnce($($Arg,)*) -> Res> {
            action: F,
            $($arg: $Arg,)*
        }

        impl<$($Arg,)* Res, F: FnOnce($($Arg,)*) -> Res> Satisfy for $satisfy<$($Arg,)* Res, F> {
            fn satisfy(self: Box<Self>) -> *mut u8 {
                let result = (self.action)($(self.$arg,)*);
                Box::into_raw(Box::new(result)) as *mut u8
            }
        }

        struct $satisfy_clone<$($Arg,)* Res> {
            action: $action_clone<$($Arg,)* Res>,
            $($arg: $Arg,)*
        }

        impl<$($Arg,)* Res> Satisfy for $satisfy_clone<$($Arg,)* Res> {
            fn satisfy(self: Box<Self>) -> *mut u8 {
                let result = self.action.borrow_mut().deref_mut()($(self.$arg,)*);
                Box::into_raw(Box::new(result)) as *mut u8
            }
        }

        #[must_use]
        pub struct $call_match<$($Arg,)* Res> {
            mock_id: usize,
            mock_type_id: usize,
            method_name: &'static str,
            type_param_ids: Vec<usize>,
            $($arg: Box<dyn MatchArg<$Arg>>,)*
            _phantom: PhantomData<Res>,
        }

        impl<$($Arg,)* Res> $call_match<$($Arg,)* Res> {
            pub fn new(
                mock_id: usize,
                mock_type_id: usize,
                method_name: &'static str,
                type_param_ids: Vec<usize>,
                $($arg: Box<dyn MatchArg<$Arg>>,)*
            ) -> Self {
                $call_match {
                    mock_id: mock_id,
                    mock_type_id: mock_type_id,
                    method_name: method_name,
                    type_param_ids: type_param_ids,
                    $($arg: $arg,)*
                    _phantom: PhantomData,
                }
            }

            fn get_args_ref(call: &Call) -> &($($Arg,)*) {
                unsafe { &mut *(call.args_ptr as *mut ($($Arg,)*)) }
            }

            fn get_args(mut call: Call) -> Box<($($Arg,)*)> {
                unsafe { Box::from_raw(call.take_args() as *mut ($($Arg,)*)) }
            }
        }

        #[must_use]
        pub struct $reaction<$($Arg,)* Res> {
            call_match: $call_match<$($Arg,)* Res>,
            action: $action_clone<$($Arg,)* Res>,
        }
        impl<$($Arg,)* Res> $reaction<$($Arg,)* Res> {
            pub fn times<C: Cardinality + 'static>(self, cardinality: C) -> $expectation_times<$($Arg,)* Res> {
                $expectation_times::new(self.call_match, self.action, Box::new(cardinality))
            }
        }

        #[must_use]
        pub struct $expectation_times<$($Arg,)* Res> {
            action: $action_clone<$($Arg,)* Res>,
            call_match: $call_match<$($Arg,)* Res>,
            cardinality: Box<dyn Cardinality>,
            count: u32,
        }

        impl<$($Arg,)* Res> $expectation_times<$($Arg,)* Res> {
            fn new(
                call_match: $call_match<$($Arg,)* Res>,
                action: $action_clone<$($Arg,)* Res>,
                cardinality: Box<dyn Cardinality>,
            ) -> Self {
                $expectation_times {
                    call_match: call_match,
                    action: action,
                    cardinality: cardinality,
                    count: 0,
                }
            }
        }

        impl<$($Arg: 'static,)* Res: 'static> Expectation for $expectation_times<$($Arg,)* Res> {
            fn call_match(&self) -> &dyn CallMatch {
                &self.call_match
            }
            fn is_satisfied(&self) -> bool {
                self.cardinality.check(self.count) == CardinalityCheckResult::Satisfied
            }
            fn satisfy(&mut self, call: Call, mock_name: &str) -> Box<Satisfy> {
                self.count += 1;
                if self.cardinality.check(self.count) == CardinalityCheckResult::Wrong {
                    panic!(
                        "{}.{} is called for the {} time, but expected to be {}",
                        mock_name,
                        self.call_match().get_method_name(),
                        format_ordinal(self.count),
                        self.cardinality.describe_upper_bound()
                    );
                }
                let ($($arg,)*) = *$call_match::<$($Arg,)* Res>::get_args(call);
                let action = self.action.clone();
                Box::new(
                    $satisfy_clone {
                        action,
                        $($arg,)*
                    }
                )
            }
            fn describe(&self) -> String {
                format!(
                    "{} must be {}, called {} times",
                    self.call_match.describe(),
                    self.cardinality.describe(),
                    self.count
                )
            }
        }

        #[must_use]
        pub struct $expectation<$($Arg,)* Res, F: FnOnce($($Arg,)*) -> Res> {
            call_match: $call_match<$($Arg,)* Res>,
            action: Option<F>,
        }

        impl<$($Arg: 'static,)* Res: 'static, F: FnOnce($($Arg,)*) -> Res + 'static> Expectation
        for $expectation<$($Arg,)* Res, F> {
            fn call_match(&self) -> &dyn CallMatch {
                &self.call_match
            }
            fn is_satisfied(&self) -> bool {
                self.action.is_none()
            }
            fn satisfy(&mut self, call: Call, mock_name: &str) -> Box<Satisfy> {
                match self.action.take() {
                    Some(action) => {
                        let ($($arg,)*) = *$call_match::<$($Arg,)* Res>::get_args(call);
                        Box::new(
                            $satisfy {
                                action,
                                $($arg,)*
                            }
                        )
                    }
                    None => {
                        panic!(
                            "{}.{} was already called earlier",
                            mock_name,
                            self.call_match().get_method_name()
                        );
                    }
                }
            }
            fn describe(&self) -> String {
                self.call_match.describe()
            }
        }

        impl<$($Arg: 'static,)* Res: 'static> $call_match<$($Arg,)* Res> {
            pub fn and_return(self, result: Res)
            -> $expectation<$($Arg,)* Res, impl FnOnce($($Arg,)*) -> Res> {
                #[allow(unused_variables)]
                $expectation {
                    call_match: self,
                    action: Some(move |$($arg,)*| result),
                }
            }

            pub fn and_panic(self, msg: String)
            -> $expectation<$($Arg,)* Res, impl FnOnce($($Arg,)*) -> Res> {
                #[allow(unused_variables)]
                $expectation {
                    call_match: self,
                    action: Some(move |$($arg,)*| panic!(msg)),
                }
            }

            pub fn and_call<F>(self, func: F)
            -> $expectation<$($Arg,)* Res, impl FnOnce($($Arg,)*) -> Res>
            where
                F: FnOnce($($Arg,)*) -> Res + 'static,
            {
                $expectation {
                    call_match: self,
                    action: Some(func),
                }
            }

            pub fn never(self) -> ExpectationNever<Self> {
                ExpectationNever { call_match: self }
            }
        }

        impl<$($Arg,)* Res: Clone + 'static> $call_match<$($Arg,)* Res> {
            pub fn and_return_clone(self, result: Res) -> $reaction<$($Arg,)* Res> {
                #[allow(unused_variables)]
                $reaction {
                    call_match: self,
                    action: Rc::new(RefCell::new(move |$($arg,)*| result.clone())),
                }
            }
        }

        impl<$($Arg,)* Res> $call_match<$($Arg,)* Res> {
            pub fn and_call_clone<F>(self, func: F) -> $reaction<$($Arg,)* Res>
            where
                F: FnMut($($Arg,)*) -> Res + 'static,
            {
                $reaction {
                    call_match: self,
                    action: Rc::new(RefCell::new(func)),
                }
            }
        }

        impl<$($Arg,)* Res: Default + 'static> $call_match<$($Arg,)* Res> {
            pub fn and_return_default(self) -> $reaction<$($Arg,)* Res> {
                #[allow(unused_variables)]
                $reaction {
                    call_match: self,
                    action: Rc::new(RefCell::new(|$($arg,)*| Res::default())),
                }
            }
        }

        impl<$($Arg,)* Res> CallMatch for $call_match<$($Arg,)* Res> {
            fn matches_args(&self, call: &Call) -> bool {
                assert!(
                    call.method_data.mock_type_id == self.mock_type_id
                        && call.method_data.method_name == self.method_name
                        && call.method_data.type_param_ids == self.type_param_ids
                );

                let __args = Self::get_args_ref(call);

                true $(&& self.$arg.matches(&__args.$n).is_ok())*
            }
            fn validate(&self, call: &Call) -> Vec<Result<(), String>> {
                let __args = Self::get_args_ref(call);
                vec![$(self.$arg.matches(&__args.$n)),*]
            }
            fn get_mock_id(&self) -> usize {
                self.mock_id
            }
            fn get_mock_type_id(&self) -> usize {
                self.mock_type_id
            }
            fn get_method_name(&self) -> &'static str {
                self.method_name
            }
            fn get_type_param_ids(&self) -> &[usize] {
                &self.type_param_ids
            }
            fn describe(&self) -> String {
                let args: &[&std::fmt::Display] = &[$(&self.$arg.describe()),*];
                format!("{}({})", self.get_method_name(), args.iter().format(", "))
            }
        }
    }
}

define_all!((CallMatch0, Reaction0, ActionClone0, Satisfy0, SatisfyClone0, Expectation0, ExpectationTimes0) {
});
define_all!((CallMatch1, Reaction1, ActionClone1, Satisfy1, SatisfyClone1, Expectation1, ExpectationTimes1) {
    (0, arg0, Arg0)
});
define_all!((CallMatch2, Reaction2, ActionClone2, Satisfy2, SatisfyClone2, Expectation2, ExpectationTimes2) {
    (0, arg0, Arg0), (1, arg1, Arg1)
});
define_all!((CallMatch3, Reaction3, ActionClone3, Satisfy3, SatisfyClone3, Expectation3, ExpectationTimes3) {
    (0, arg0, Arg0), (1, arg1, Arg1), (2, arg2, Arg2)
});
define_all!((CallMatch4, Reaction4, ActionClone4, Satisfy4, SatisfyClone4, Expectation4, ExpectationTimes4) {
    (0, arg0, Arg0), (1, arg1, Arg1), (2, arg2, Arg2), (3, arg3, Arg3)
});

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
        Sequence {
            expectations: Vec::new(),
        }
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
    fn satisfy(&mut self, call: Call, mock_name: &str) -> Box<Satisfy> {
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
    where
        &'static T: Mocked,
    {
        Self::create_mock::<<&'static T as Mocked>::MockImpl>(int)
    }

    pub fn create_named_mock_for<T: ?Sized>(
        int: &Rc<RefCell<Self>>,
        name: String,
    ) -> <&'static T as Mocked>::MockImpl
    where
        &'static T: Mocked,
    {
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
    where
        &'static T: Mocked,
    {
        ScenarioInternals::create_mock_for::<T>(&self.internals)
    }

    pub fn create_named_mock_for<T: ?Sized>(&self, name: String) -> <&'static T as Mocked>::MockImpl
    where
        &'static T: Mocked,
    {
        ScenarioInternals::create_named_mock_for::<T>(&self.internals, name)
    }

    pub fn expect<C: Expectation + 'static>(&self, call: C) {
        self.internals
            .borrow_mut()
            .expectations
            .push(Box::new(call));
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
                let mock_name = mock_names
                    .get(&expectation.call_match().get_mock_id())
                    .unwrap();
                s.push_str(&format!("`{}.{}`\n", mock_name, expectation.describe()));
            }
            panic!(s);
        }
    }
}

impl Default for Scenario {
    fn default() -> Self {
        Self::new()
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

        self.verify_expectations();
    }
}

pub struct ScenarioHandle {
    internals: Weak<RefCell<ScenarioInternals>>,
}

impl ScenarioHandle {
    pub fn new(scenario_int: Weak<RefCell<ScenarioInternals>>) -> Self {
        Self {
            internals: scenario_int,
        }
    }

    pub fn create_mock<T: Mock>(&self) -> T {
        ScenarioInternals::create_mock::<T>(&self.get_internals())
    }

    pub fn create_named_mock<T: Mock>(&self, name: String) -> T {
        ScenarioInternals::create_named_mock::<T>(&self.get_internals(), name)
    }

    pub fn create_mock_for<T: ?Sized>(&self) -> <&'static T as Mocked>::MockImpl
    where
        &'static T: Mocked,
    {
        ScenarioInternals::create_mock_for::<T>(&self.get_internals())
    }

    pub fn create_named_mock_for<T: ?Sized>(&self, name: String) -> <&'static T as Mocked>::MockImpl
    where
        &'static T: Mocked,
    {
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

pub struct MethodData {
    /// Unique ID of mock object
    pub mock_id: usize,

    /// Unique ID of mock class
    pub mock_type_id: usize,

    /// Called method name
    pub method_name: &'static str,

    /// Type parameters of generic method
    pub type_param_ids: Vec<usize>,
}

macro_rules! define_verify {
    (
        $verify:ident { $(($n:tt, $arg:ident, $Arg:ident)),* }
    ) => {
        pub fn $verify<$($Arg: DebugOnStable,)* Res>(
            &mut self, method_data: MethodData$(, $arg: $Arg)*
        ) -> impl FnOnce() -> Res {
            let args = Box::new(($($arg,)*));
            let args_ptr: *const u8 = ::std::boxed::Box::into_raw(args) as *const u8;
            fn destroy<$($Arg,)*>(args_to_destroy: *const u8) {
                unsafe { Box::from_raw(args_to_destroy as *mut ($($Arg,)*)) };
            };
            fn format_args<$($Arg: DebugOnStable,)*>(args_ptr: *const u8) -> String {
                let __args_ref: &($($Arg,)*) = unsafe { ::std::mem::transmute(args_ptr) };
                let args_debug: &[&std::fmt::Debug] = &[$(&dbg(&__args_ref.$n)),*];
                format!("{:?}", args_debug.iter().format(", "))
            };
            let call = Call {
                method_data: method_data,
                args_ptr: args_ptr,
                destroy: destroy::<$($Arg,)*>,
                format_args: format_args::<$($Arg,)*>,
            };
            let action = self.verify(call);
            move || {
                let result_ptr: *mut u8 = action.satisfy();
                let result: Box<Res> = unsafe { Box::from_raw(result_ptr as *mut Res) };
                *result
            }
        }
    }
}

impl ScenarioInternals {
    define_verify!(verify0 { });
    define_verify!(verify1 { (0, arg0, Arg0) });
    define_verify!(verify2 { (0, arg0, Arg0), (1, arg1, Arg1) });
    define_verify!(verify3 { (0, arg0, Arg0), (1, arg1, Arg1), (2, arg2, Arg2) });
    define_verify!(verify4 { (0, arg0, Arg0), (1, arg1, Arg1), (2, arg2, Arg2), (3, arg3, Arg3) });

    /// Verify call performed on mock object
    /// Returns closure which returns result upon call.
    /// Closure returned instead of actual result, because expectation may
    /// use user-provided closure as action, and that closure may want to
    /// use scenario object to create mocks or establish expectations, so
    /// we need to release scenario borrow before calling expectation action.
    fn verify(&mut self, call: Call) -> Box<Satisfy> {
        for expectation in self.expectations.iter_mut().rev() {
            if expectation.call_match().matches(&call) {
                let mock_name = self
                    .mock_names
                    .get(&call.method_data.mock_id)
                    .unwrap()
                    .clone();
                return expectation.satisfy(call, &mock_name);
            }
        }

        // No expectations exactly matching call are found. However this may be
        // because of unexpected argument values. So check active expectations
        // with matching target (i.e. mock and method) and validate arguments.
        let mock_name = self.mock_names.get(&call.method_data.mock_id).unwrap();

        let mut msg = String::new();
        msg.write_str("\n\n").unwrap();
        write!(
            &mut msg,
            concat!(
                colored!(red: "error:"),
                " ",
                colored!(bold: "unexpected call to `{}.{}({})`\n\n")
            ),
            mock_name,
            call.method_data.method_name,
            (call.format_args)(call.args_ptr)
        )
        .unwrap();

        if self.expectations.is_empty() {
            msg.push_str("no call are expected");
            panic!(msg);
        }

        let mut target_first_match = true;
        for expectation in self.expectations.iter().rev() {
            if !expectation.is_satisfied() && expectation.call_match().matches_method(&call) {
                if target_first_match {
                    write!(
                        &mut msg,
                        concat!(
                            colored!(green: "note: "),
                            "here are active expectations for {}.{}\n"
                        ),
                        mock_name, call.method_data.method_name
                    )
                    .unwrap();
                    target_first_match = false;
                }

                write!(
                    &mut msg,
                    "\n  expectation `{}.{}`:\n",
                    mock_name,
                    expectation.describe()
                )
                .unwrap();
                for (index, res) in expectation.call_match().validate(&call).iter().enumerate() {
                    match *res {
                        Err(ref err) => write!(
                            &mut msg,
                            concat!("    arg #{}: ", colored!(bold: "{}"), "\n"),
                            index, err
                        )
                        .unwrap(),
                        Ok(()) => (),
                    }
                }
            }
        }

        if target_first_match {
            write!(
                &mut msg,
                concat!(
                    colored!(green: "note: "),
                    "there are no active expectations for {}.{}\n"
                ),
                mock_name, call.method_data.method_name
            )
            .unwrap();
        }

        let mut method_first_match = true;
        for expectation in self.expectations.iter().rev() {
            if !expectation.is_satisfied()
                && !expectation.call_match().matches_target(&call)
                && expectation.call_match().matches_method(&call)
                && expectation.call_match().matches_args(&call)
            {
                if method_first_match {
                    msg.push_str(concat!(
                        colored!(green: "note: "),
                        "there are matching expectations for another mock objects\n"
                    ));
                    method_first_match = false;
                }

                let other_mock_id = &expectation.call_match().get_mock_id();
                let other_mock_name = self.mock_names.get(other_mock_id).unwrap();
                write!(
                    &mut msg,
                    concat!("\n  expectation `", colored!(bold: "{}"), ".{}`\n"),
                    other_mock_name,
                    expectation.describe()
                )
                .unwrap();
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
