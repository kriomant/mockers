#![feature(proc_macro)]

///! Test that plugin can generate mock for
///! trait placed in some other module.

extern crate mockers;
extern crate mockers_derive;

use mockers_derive::mock;

mod nested {
    pub trait A {
        fn foo(&self);
    }
}

mock!{
    AMock,
    nested,
    trait A {
        fn foo(&self);
    }
}
