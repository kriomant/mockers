use std::panic::AssertUnwindSafe;
use std::rc::Rc;

use mockers::matchers::{lt, ANY};
use mockers::{Scenario, Sequence};
use mockers_derive::{mock, mocked};

#[mocked]
pub trait A {
    fn foo(&self);
    fn bar(&self, arg: u32);
    fn baz(&self) -> u32;
    fn modify(&mut self);
    fn ask(&self, arg: u32) -> u32;
    fn consume(self);
    fn consume_result(&self) -> String;
    fn consume_arg(&self, arg: String) -> String;
    fn consume_rc(&self, arg: Rc<usize>);
}

mock! {
    AMockByMacro,
    self,
    trait A {
        fn foo(&self);
        fn bar(&self, arg: u32);
        fn baz(&self) -> u32;
        fn modify(&mut self);
        fn ask(&self, arg: u32) -> u32;
        fn consume(self);
        fn consume_result(&self) -> String;
        fn consume_arg(&self, arg: String) -> String;
        fn consume_rc(&self, arg: Rc<usize>);
    }
}

#[test]
#[should_panic(expected = "unexpected call to `A#0.foo()`")]
fn test_unit() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();
    scenario.expect(handle.bar_call(2).and_return(()));
    mock.foo();
}

#[test]
fn test_return() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();
    scenario.expect(handle.baz_call().and_return(2));
    assert_eq!(2, mock.baz());
}

#[test]
#[should_panic(expected = "4 is not less than 3")]
fn test_arg_match_failure() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();
    scenario.expect(handle.bar_call(lt(3)).and_return(()));
    mock.bar(4);
}

#[test]
fn test_arg_match_success() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();
    scenario.expect(handle.bar_call(lt(3)).and_return(()));
    mock.bar(2);
}

#[test]
#[should_panic(expected = "Some expectations are not satisfied:\n`A#0.bar(_)`\n")]
fn test_expected_call_not_performed() {
    let scenario = Scenario::new();
    let (_mock, handle) = scenario.create_mock_for::<A>();
    scenario.expect(handle.bar_call(ANY).and_return(()));
}

#[test]
#[should_panic(expected = "boom!")]
fn test_panic_result() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();
    scenario.expect(handle.foo_call().and_panic("boom!".to_owned()));
    mock.foo();
}

#[test]
fn test_mut_self_method() {
    let scenario = Scenario::new();
    let (mut mock, handle) = scenario.create_mock_for::<A>();
    scenario.expect(handle.modify_call().and_return(()));
    mock.modify();
}

#[test]
fn test_value_self_method() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();
    scenario.expect(handle.consume_call().and_return(()));
    mock.consume();
}

#[test]
#[should_panic(expected = "unexpected call to `amock.foo()`")]
fn test_named_mock() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_named_mock_for::<A>("amock".to_owned());
    scenario.expect(handle.bar_call(2).and_return(()));
    mock.foo();
}

/// Test that when test is failed, then remaining scenario
/// expectations are not checked and don't cause panic-during-drop
/// which will lead to ugly failure with not very useful message.
#[test]
#[should_panic(expected = "caboom!")]
fn test_failed_with_remaining_expectations() {
    let scenario = Scenario::new();
    let (_mock, handle) = scenario.create_mock_for::<A>();

    // This expectation will never be satisfied.
    scenario.expect(handle.bar_call(2).and_return(()));
    panic!("caboom!");
}

#[test]
fn test_expect_and_call() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    // This expectation will never be satisfied.
    scenario.expect(handle.ask_call(2).and_call(|arg| arg + 1));
    assert_eq!(mock.ask(2), 3);
}

#[test]
fn test_expect_is_unordered() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return(()));
    scenario.expect(handle.bar_call(2).and_return(()));

    mock.bar(2);
    mock.foo();
}

#[test]
#[should_panic(expected = "A#0.foo was already called earlier")]
fn test_expect_consumes_one_call_only() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return(()));

    mock.foo();
    mock.foo();
}

#[test]
fn test_never_satisfied() {
    let scenario = Scenario::new();
    let (_mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().never());
}

#[test]
fn test_never_on_call_with_args() {
    let scenario = Scenario::new();
    let (_mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.bar_call(ANY).never());
}

#[test]
#[should_panic(expected = "A#0.foo should never be called")]
fn test_never_not_satisfied() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().never());

    mock.foo();
}

#[test]
fn test_consume_result() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    let result = "ho-ho".to_owned();
    scenario.expect(handle.consume_result_call().and_return(result));

    assert_eq!(mock.consume_result(), "ho-ho");
}

#[test]
fn test_consume_call_result() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    let result = "ho-ho".to_owned();
    scenario.expect(handle.consume_result_call().and_call(move || result));

    assert_eq!(mock.consume_result(), "ho-ho");
}

#[test]
fn test_consume_argument() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.consume_arg_call(ANY).and_call(|arg| arg));

    let arg = "ho-ho".to_owned();
    assert_eq!(mock.consume_arg(arg), "ho-ho");
}

#[test]
fn test_arguments_are_dropped_on_panic() {
    let scenario = Scenario::new();
    let (mock, _) = scenario.create_mock_for::<A>();

    let arg = Rc::new(0);
    let weak = Rc::downgrade(&arg);
    assert!(weak.upgrade().is_some());

    let mock_ref = AssertUnwindSafe(&mock);
    let result = std::panic::catch_unwind(|| {
        // This will cause panic, because there is no matching
        // expectation. Argument must be dropped during unwinding.
        mock_ref.consume_rc(arg);
    });
    assert!(result.is_err());
    assert!(weak.upgrade().is_none());
}

#[test]
#[should_panic(expected = "`A#0.foo() must be called exactly 2 times, called 1 times`")]
fn test_checkpoint() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.foo_call().and_return_clone(()).times(2));

    mock.foo();

    scenario.checkpoint();

    mock.foo();
}

#[test]
fn test_create_mock() {
    let scenario = Scenario::new();
    let _mock = scenario.create_mock::<AMockByMacro>();
}

#[test]
#[should_panic(expected = "unexpected call to `A#0.bar(12)`")]
fn test_format_args() {
    let scenario = Scenario::new();
    let (mock, _) = scenario.create_mock_for::<A>();

    mock.bar(12);
}

// When no matching expectation found for call, expectations
// for other mock object of the same type must be checked.
#[test]
// Message without ANSI codes is "expectation `A#0.bar(12)`"
#[should_panic(expected = "expectation `\x1b[1mA#0\x1b[0m.bar(12)`")]
fn test_check_other_mock_object_expectations() {
    let scenario = Scenario::new();
    let (_mock0, handle0) = scenario.create_mock_for::<A>();
    let (mock1, _) = scenario.create_mock_for::<A>();

    scenario.expect(handle0.bar_call(12).and_return(()));

    mock1.bar(12);
}

#[test]
fn test_sequence() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    let mut seq = Sequence::new();
    seq.expect(handle.foo_call().and_return(()));
    seq.expect(handle.bar_call(4).and_return(()));
    scenario.expect(seq);

    mock.foo();
    mock.bar(4);
}

#[test]
#[should_panic(expected = "unexpected call to `A#0.bar(4)`")]
fn test_sequence_invalid_order() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    let mut seq = Sequence::new();
    seq.expect(handle.foo_call().and_return(()));
    seq.expect(handle.bar_call(4).and_return(()));
    scenario.expect(seq);

    mock.bar(4);
    mock.foo();
}

#[test]
fn test_sequence_times() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    let mut seq = Sequence::new();
    seq.expect(handle.foo_call().and_return_clone(()).times(2));
    seq.expect(handle.bar_call(4).and_return(()));
    scenario.expect(seq);

    mock.foo();
    mock.foo();
    mock.bar(4);
}

#[test]
#[should_panic(expected = "unexpected call to `A#0.bar(4)`")]
fn test_sequence_times_invalid() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    let mut seq = Sequence::new();
    seq.expect(handle.foo_call().and_return_clone(()).times(2));
    seq.expect(handle.bar_call(4).and_return(()));
    scenario.expect(seq);

    mock.foo();
    mock.bar(4);
}

#[test]
fn test_return_default() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<A>();

    scenario.expect(handle.baz_call().and_return_default().times(1));

    assert_eq!(mock.baz(), 0);
}
