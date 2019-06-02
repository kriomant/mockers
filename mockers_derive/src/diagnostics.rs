extern crate proc_macro;

#[cfg(feature = "nightly")]
pub use proc_macro::{Diagnostic, Level};

#[cfg(not(feature = "nightly"))]
#[derive(Debug)]
pub enum Level {
    Error,
    Warning,
}

#[cfg(not(feature = "nightly"))]
pub struct Diagnostic {
    level: Level,
    msg: String,
}

#[cfg(not(feature = "nightly"))]
impl<'a> Diagnostic {
    pub fn new<T: Into<String>>(level: Level, msg: T) -> Self {
        Self {
            level,
            msg: msg.into(),
        }
    }

    pub fn spanned<T: Into<String>>(_: proc_macro::Span, level: Level, msg: T) -> Self {
        Self {
            level,
            msg: msg.into(),
        }
    }

    pub fn emit(&self) {
        match self.level {
            Level::Error => panic!("error in mockers: {}", self.msg),
            Level::Warning => eprintln!("warning in mockers: {}", self.msg),
        }
    }
}
