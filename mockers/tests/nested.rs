///! Test that plugin can generate mock for
///! trait placed in some other module.
use mockers_derive::mock;

mod nested {
    pub trait A {
        fn foo(&self);
    }
}

mock! {
    AMock,
    nested,
    trait A {
        fn foo(&self);
    }
}
