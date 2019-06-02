use super::CallMatch0;

pub trait CloneMock<Mock>: Sized {
    fn clone(&self) -> CallMatch0<Mock>;
}

/// Implements `Clone` for mock object.
///
/// Sometimes it is needed for mock to be clonable. Say you have
/// following trait and function accepting this trait you need to test:
///
/// ```rust,ignore
/// #[mocked]
/// pub trait A {
///     fn foo(&self, a: u32);
/// }
///
/// fn target<AC: A + Clone>(a: AC) {
///     let clone = a.clone();
///     clone.foo(2);
/// }
/// ```
///
/// There are two forms of macro invokation.
/// First one with mock struct name as single argument mocks `clone` method,
/// so you may specify arbitrary expectations and reactions on it:
///
/// ```rust,ignore
/// mock_clone!(AMock);
///
/// #[test]
/// fn test_clone_mock() {
///     let scenario = Scenario::new();
///     let mock = scenario.create_mock_for::<dyn A>();
///     let mock_clone = scenario.create_mock_for::<dyn A>();
///
///     scenario.expect(mock_clone.foo(2).and_return_default().times(1));
///     scenario.expect(mock.clone().and_return(mock_clone));
///
///     target(mock);
/// }
/// ```
///
/// Please note that you must specify mock name, not name of mocked trait. This is
/// limitation of current macro_rules system. If you mocked trait using `derive`
/// attribute, then just append "Mock" to trait name.
///
/// Second form accepts one additional parameter - strategy, which specifies how
/// cloned mock should behave. Currently there is only one strategy - "share_expectations",
/// which means that all mock clones are indistinguishable and expectations set on one
/// of them may be satisfied by calls made on another one. This is very useful when
/// mocked trait behaves like handle to some real implementation.
///
/// ```rust,ignore
/// mock_clone!(AMock, share_expectations);
///
/// #[test]
/// fn test_shared() {
///     let scenario = Scenario::new();
///     let mock = scenario.create_mock_for::<dyn A>();
///
///     scenario.expect(mock.foo(2).and_return_default().times(1));
///
///     target(mock);
/// }
/// ```
#[macro_export]
macro_rules! mock_clone {
    ($mock_name:ident, $handle_name:ident) => {
        #[cfg(test)]
        impl Clone for $mock_name {
            fn clone(&self) -> Self {
                let method_data = ::mockers::MethodData {
                    mock_id: self.mock_id,
                    mock_type_id: 0usize,
                    method_name: "clone",
                    type_param_ids: vec![],
                };
                let action = self.scenario.borrow_mut().verify0(method_data);
                action()
            }
        }

        #[cfg(test)]
        impl $crate::CloneMock<$mock_name> for $handle_name {
            #[allow(dead_code)]
            fn clone(&self) -> ::mockers::CallMatch0<$mock_name> {
                ::mockers::CallMatch0::new(self.mock_id, 0usize, "clone", vec![])
            }
        }
    };

    ($mock_name:ident, $handle_name:ident, share_expectations) => {
        #[cfg(test)]
        impl Clone for $mock_name {
            fn clone(&self) -> Self {
                use $crate::Mock;
                $mock_name::new(self.mock_id, self.scenario.clone())
            }
        }
    };
}
