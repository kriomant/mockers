use mockers::matchers::ANY;
use mockers::Scenario;
///! Test that mockers can mock several traits using one mock.
///! In particular, it should work for mocking inherited traits.
use mockers_derive;

/// Test mocking of inherited trait using `mocked`.
mod derive_inherited_trait {
    use super::*;
    use mockers_derive::mocked;

    #[mocked(module = "::derive_inherited_trait")]
    pub trait A {
        fn foo(&self, a: u32);
    }

    #[mocked(refs = "A => ::derive_inherited_trait::A")]
    pub trait B: A {
        fn bar(&self, b: u32);
    }

    #[test]
    fn test() {
        let scenario = Scenario::new();
        let (mock, _) = scenario.create_mock::<BMock>();

        scenario.expect(mock.foo_call(ANY).and_return_default().times(1));
        scenario.expect(mock.bar_call(ANY).and_return_default().times(1));

        mock.foo(3);
        mock.bar(4);
    }
}

/// Test mocking of inherited trait in different modules using `mocked`.
mod derive_inherited_trait_different_modules {
    use super::*;

    mod a {
        use mockers_derive::mocked;

        #[mocked(module = "::derive_inherited_trait_different_modules::a")]
        pub trait A {
            fn foo(&self, a: u32);
        }
    }

    mod b {
        use mockers_derive::mocked;

        #[mocked(refs = "super::a::A => ::derive_inherited_trait_different_modules::a::A")]
        pub trait B: super::a::A {
            fn bar(&self, b: u32);
        }
    }

    #[test]
    fn test() {
        use self::a::A;
        use self::b::B;

        let scenario = Scenario::new();
        let (mock, _) = scenario.create_mock::<b::BMock>();

        scenario.expect(mock.foo_call(ANY).and_return_default().times(1));
        scenario.expect(mock.bar_call(ANY).and_return_default().times(1));

        mock.foo(3);
        mock.bar(4);
    }
}

// Test mocking of inherited trait.
mod inherited_trait {
    use super::*;
    use mockers_derive::mock;

    pub trait A {
        fn foo(&self, a: u32);
    }

    pub trait B: A {
        fn bar(&self, b: u32);
    }

    mock! {
        BMock,

        self,
        trait A {
            fn foo(&self, a: u32);
        },

        self,
        trait B {
            fn bar(&self, b: u32);
        }
    }

    #[test]
    fn test() {
        let scenario = Scenario::new();
        let (mock, _) = scenario.create_mock::<BMock>();

        scenario.expect(mock.foo_call(ANY).and_return_default().times(1));
        scenario.expect(mock.bar_call(ANY).and_return_default().times(1));

        mock.foo(3);
        mock.bar(4);
    }
}

// Test creating mock for several independent traits at once.
mod multi_trait {
    use super::*;
    use mockers_derive::mock;

    pub trait A {
        fn foo(&self, a: u32);
    }

    pub trait B {
        fn bar(&self, b: u32);
    }

    mock! {
        ABMock,

        self,
        trait A {
            fn foo(&self, a: u32);
        },

        self,
        trait B {
            fn bar(&self, b: u32);
        }
    }

    fn accept_cd<T: A + B>(t: T) {
        t.foo(1);
        t.bar(2);
    }

    #[test]
    fn test() {
        let scenario = Scenario::new();
        let (mock, _) = scenario.create_mock::<ABMock>();

        scenario.expect(mock.foo_call(ANY).and_return_default().times(1));
        scenario.expect(mock.bar_call(ANY).and_return_default().times(1));

        accept_cd(mock);
    }
}

// Test that it is possible to specify parent trait when using `mock!`.
/// It is currently not used, but may be used in the future, so syntax
/// should be allowed.
mod inherited_trait_with_specified_parent {
    use mockers_derive::mock;

    pub trait A {
        fn foo(&self, a: u32);
    }

    pub trait B: A {
        fn bar(&self, b: u32);
    }

    mock! {
        BMock,

        self,
        trait A {
            fn foo(&self, a: u32);
        },

        self,
        trait B: A {
            fn bar(&self, b: u32);
        }
    }
}
