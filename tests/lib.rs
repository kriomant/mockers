#![feature(plugin)]
#![plugin(mockers_macros)]

extern crate mockers;

use mockers::Scenario;
use mockers::matchers::{ANY, lt};

pub trait A {
    fn foo(&self);
    fn bar(&self, arg: u32);
    fn baz(&self) -> u32;
    fn modify(&mut self);
    fn ask(&self, arg: u32) -> u32;
    fn consume(self);
    fn consume_result(&self) -> String;
    fn consume_arg(&self, arg: String) -> String;
}

mock!{
    AMock,
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
    }
}

#[test]
#[should_panic(expected="Unexpected call to `A#0.foo`\n\n\
                         There are no active expectations for same method call")]
fn test_unit() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.bar_call(2).and_return(()));
    mock.foo();
}

#[test]
fn test_return() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.baz_call().and_return(2));
    assert_eq!(2, mock.baz());
}


#[test]
#[should_panic(expected="4 is not less than 3")]
fn test_arg_match_failure() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.bar_call(lt(3)).and_return(()));
    mock.bar(4);
}

#[test]
fn test_arg_match_success() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.bar_call(lt(3)).and_return(()));
    mock.bar(2);
}


#[test]
#[should_panic(expected="Expected calls are not performed:\n`A#0.bar(_)`\n")]
fn test_expected_call_not_performed() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.bar_call(ANY).and_return(()));
}


#[test]
#[should_panic(expected="boom!")]
fn test_panic_result() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.foo_call().and_panic("boom!".to_owned()));
    mock.foo();
}

#[test]
fn test_mut_self_method() {
    let mut scenario = Scenario::new();
    let mut mock = scenario.create_mock::<A>();
    scenario.expect(mock.modify_call().and_return(()));
    mock.modify();
}

#[test]
fn test_value_self_method() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.consume_call().and_return(()));
    mock.consume();
}

#[test]
#[should_panic(expected="Unexpected call to `amock.foo`\n\n\
                         There are no active expectations for same method call")]
fn test_named_mock() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_named_mock::<A>("amock".to_owned());
    scenario.expect(mock.bar_call(2).and_return(()));
    mock.foo();
}

/// Test that when test is failed, then remaining scenario
/// expectations are not checked and don't cause panic-during-drop
/// which will lead to ugly failure with not very useful message.
#[test]
#[should_panic(expected="caboom!")]
fn test_failed_with_remaining_expectations() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();

    // This expectation will never be satisfied.
    scenario.expect(mock.bar_call(2).and_return(()));
    panic!("caboom!");
}

#[test]
fn test_expect_and_call() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();

    // This expectation will never be satisfied.
    scenario.expect(mock.ask_call(2).and_call(|arg| { arg+1 }));
    assert_eq!(mock.ask(2), 3);
}

#[test]
fn test_expect_is_unordered() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();

    scenario.expect(mock.foo_call().and_return(()));
    scenario.expect(mock.bar_call(2).and_return(()));

    mock.bar(2);
    mock.foo();
}

#[test]
#[should_panic(expect="A#0.foo was already called earlier")]
fn test_expect_consumes_one_call_only() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();

    scenario.expect(mock.foo_call().and_return(()));

    mock.foo();
    mock.foo();
}

#[test]
fn test_never_satisfied() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();

    scenario.expect(mock.foo_call().never());
}

#[test]
#[should_panic(expect="A#0.foo should never be called")]
fn test_never_not_satisfied() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();

    scenario.expect(mock.foo_call().never());

    mock.foo();
}

#[test]
fn test_consume_result() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();

    let result = "ho-ho".to_owned();
    scenario.expect(mock.consume_result_call().and_return(result));

    assert_eq!(mock.consume_result(), "ho-ho");
}

#[test]
fn test_consume_call_result() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();

    let result = "ho-ho".to_owned();
    scenario.expect(mock.consume_result_call().and_call(move || { result }));

    assert_eq!(mock.consume_result(), "ho-ho");
}

#[test]
fn test_consume_argument() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();

    scenario.expect(mock.consume_arg_call(ANY).and_call(|arg| { arg }));

    let arg = "ho-ho".to_owned();
    assert_eq!(mock.consume_arg(arg), "ho-ho");
}

