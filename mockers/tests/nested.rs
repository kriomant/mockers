#![feature(plugin)]
#![plugin(mockers_macros)]

///! Test that plugin can generate mock for
///! trait placed in some other module.

extern crate mockers;

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
