use std::boxed::FnBox;

pub struct BoxFn0<T>(Box<dyn FnBox() -> T>);
impl<T> BoxFn0<T> {
    pub fn new<F: 'static + FnOnce() -> T>(f: F) -> Self {
        BoxFn0(Box::new(f))
    }
    pub fn call(self) -> T {
        self.0()
    }
}
