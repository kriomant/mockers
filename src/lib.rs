use std::marker::PhantomData;
use std::rc::Rc;
use std::cell::RefCell;

pub trait CheckCall {
    fn check_call(self: Box<Self>, args: *const u8) -> *mut u8;
    fn get_mock_id(&self) -> usize;
    fn get_method_id(&self) -> usize;
}

#[must_use]
pub struct CallMatch0<Res> {
    mock_id: usize,
    method_id: usize,

    _phantom: PhantomData<Res>,
}
impl<Res> CallMatch0<Res> {
    pub fn new(mock_id: usize, method_id: usize) -> Self {
        CallMatch0 {
            mock_id: mock_id,
            method_id: method_id,
            _phantom: PhantomData
        }
    }
}

#[must_use]
pub struct Expectation0<Res> {
    call_match: CallMatch0<Res>,
    result: Res,
}
impl<Res> Expectation0<Res> {
    fn check(self) -> Res { self.result }
}
impl<Res> CheckCall for Expectation0<Res> {
    fn check_call(self: Box<Self>, _args: *const u8) -> *mut u8 {
        //let args_tuple: &() = unsafe { std::mem::transmute(args) };
        let result = self.check();
        Box::into_raw(Box::new(result)) as *mut u8
    }
    fn get_mock_id(&self) -> usize { self.call_match.mock_id }
    fn get_method_id(&self) -> usize { self.call_match.method_id }
}
impl<Res> CallMatch0<Res> {
    pub fn and_return(self, result: Res) -> Expectation0<Res> {
        Expectation0 { call_match: self, result: result }
    }
}

#[must_use]
pub struct CallMatch1<Arg0, Res> {
    mock_id: usize,
    method_id: usize,
    arg0: MatchArg<Arg0>,

    _phantom: PhantomData<Res>,
}
impl<Arg0, Res> CallMatch1<Arg0, Res> {
    pub fn new(mock_id: usize, method_id: usize, arg0: MatchArg<Arg0>) -> Self {
        CallMatch1 {
            mock_id: mock_id,
            method_id: method_id,
            arg0: arg0,
            _phantom: PhantomData
        }
    }
}

#[must_use]
pub struct Expectation1<Arg0, Res> {
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
    pub fn and_return(self, result: Res) -> Expectation1<Arg0, Res> {
        Expectation1 { call_match: self, result: result }
    }
}

#[must_use]
pub struct CallMatch2<Arg0, Arg1, Res> {
    mock_id: usize,
    method_id: usize,
    arg0: MatchArg<Arg0>,
    arg1: MatchArg<Arg1>,

    _phantom: PhantomData<Res>,
}
impl<Arg0, Arg1, Res> CallMatch2<Arg0, Arg1, Res> {
    pub fn new(mock_id: usize, method_id: usize,
               arg0: MatchArg<Arg0>, arg1: MatchArg<Arg1>) -> Self {
        CallMatch2 {
            mock_id: mock_id,
            method_id: method_id,
            arg0: arg0,
            arg1: arg1,
            _phantom: PhantomData
        }
    }
}

#[must_use]
pub struct Expectation2<Arg0, Arg1, Res> {
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
    pub fn and_return(self, result: Res) -> Expectation2<Arg0, Arg1, Res> {
        Expectation2 { call_match: self, result: result }
    }
}

pub type MatchArg<T> = Box<Fn(&T) -> Result<(), String>>;

pub trait IntoMatchArg<T> {
    fn into_match_arg(self) -> MatchArg<T>;
}

impl<T> IntoMatchArg<T> for MatchArg<T> {
    fn into_match_arg(self) -> Self { self }
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

pub trait Mock {
    fn new(id: usize, scenario_int: Rc<RefCell<ScenarioInternals>>) -> Self;
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

    pub fn create_mock<T: Mock>(&mut self) -> T {
        T::new(self.get_next_mock_id(), self.internals.clone())
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
        let event = self.events.remove(0);
        if event.get_mock_id() != mock_id || event.get_method_id() != method_id {
            panic!("Unexpected event");
        }
        event.check_call(args_ptr)
    }
}
