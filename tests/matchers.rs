#![feature(plugin)]
#![plugin(mockers_macros)]

#[macro_use(arg, check)]
extern crate mockers;

use mockers::Scenario;
use mockers::matchers::*;

pub trait A {
    fn bar(&self, arg: u32);
    fn noarg(&self);
    fn num(&self, arg: u32);
    fn cmplx(&self, maybe: Option<u32>);
}

mock!{
    AMock,
    self,
    trait A {
        fn bar(&self, arg: u32);
        fn noarg(&self);
        fn num(&self, arg: u32);
        fn cmplx(&self, maybe: Option<u32>);
    }
}


#[test]
fn test_any_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.bar_call(ANY).and_return(()));
    mock.bar(2);
}


#[test]
fn test_eq_matcher_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(eq(2)).and_return(()));
    mock.num(2);
}

#[test]
#[should_panic(expected="3 is not equal to 2")]
fn test_eq_matcher_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(eq(2)).and_return(()));
    mock.num(3);
}


#[test]
fn test_ne_matcher_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(ne(2)).and_return(()));
    mock.num(3);
}

#[test]
#[should_panic(expected="2 is equal to 2")]
fn test_ne_matcher_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(ne(2)).and_return(()));
    mock.num(2);
}


#[test]
fn test_lt_matcher_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(lt(2)).and_return(()));
    mock.num(1);
}

#[test]
#[should_panic(expected="2 is not less than 2")]
fn test_lt_matcher_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(lt(2)).and_return(()));
    mock.num(2);
}


#[test]
fn test_le_matcher_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(le(2)).and_return(()));
    scenario.expect(mock.num_call(le(2)).and_return(()));
    mock.num(1);
    mock.num(2);
}

#[test]
#[should_panic(expected="3 is not less than or equal to 2")]
fn test_le_matcher_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(le(2)).and_return(()));
    mock.num(3);
}


#[test]
fn test_gt_matcher_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(gt(2)).and_return(()));
    mock.num(3);
}

#[test]
#[should_panic(expected="2 is not greater than 2")]
fn test_gt_matcher_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(gt(2)).and_return(()));
    mock.num(2);
}


#[test]
fn test_ge_matcher_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(ge(2)).and_return(()));
    scenario.expect(mock.num_call(ge(2)).and_return(()));
    mock.num(2);
    mock.num(3);
}

#[test]
#[should_panic(expected="1 is not greater than or equal to 2")]
fn test_ge_matcher_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(ge(2)).and_return(()));
    mock.num(1);
}


#[test]
fn test_not_matcher_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(not(ge(2))).and_return(()));
    mock.num(1);
}

#[test]
#[should_panic(expected="2 matches (but shouldn\\'t): lt(2)")]
fn test_not_matcher_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(not(ge(2))).and_return(()));
    mock.num(2);
}


// Special matcher which panics when called.
// It is used to verify that logical operators on
// matchers are short-circuit.
struct UnreachableMatcher;
impl<T> mockers::MatchArg<T> for UnreachableMatcher {
    fn matches(&self, _: &T) -> Result<(), String> {
        unreachable!();
    }

    fn describe(&self) -> String {
        "unreachable".to_owned()
    }
}


#[test]
fn test_and_matcher_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(and(gt(2), lt(5))).and_return(()));
    mock.num(3);
}

#[test]
#[should_panic(expected="1 is not greater than 2")]
fn test_and_matcher_short_circuit() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(and(gt(2), UnreachableMatcher)).and_return(()));
    mock.num(1);
}

#[test]
#[should_panic(expected="6 is not less than 5")]
fn test_and_matcher_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(and(gt(2), lt(5))).and_return(()));
    mock.num(6);
}


#[test]
fn test_or_matcher_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(or(lt(2), gt(5))).and_return(()));
    mock.num(1);
}

#[test]
fn test_or_matcher_short_circuit() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(or(lt(2), UnreachableMatcher)).and_return(()));
    mock.num(1);
}

#[test]
#[should_panic(expected="4 is not greater than 5")]
fn test_or_matcher_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.num_call(or(lt(2), gt(5))).and_return(()));
    mock.num(4);
}


#[test]
fn test_arg_macro_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.cmplx_call(arg!(Some(_))).and_return(()));
    mock.cmplx(Some(3));
}

#[test]
#[should_panic(expected="None isn\\'t matched by Some(_)")]
fn test_arg_macro_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.cmplx_call(arg!(Some(_))).and_return(()));
    mock.cmplx(None);
}


#[test]
fn test_check_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.cmplx_call(check(|t:&Option<u32>| t.is_some())).and_return(()));
    mock.cmplx(Some(3));
}

#[test]
#[should_panic(expected="<custom function>")]
fn test_check_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.cmplx_call(check(|t:&Option<u32>| t.is_some())).and_return(()));
    mock.cmplx(None);
}


#[test]
fn test_check_macro_match() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.cmplx_call(check!(|t:&Option<u32>| t.is_some())).and_return(()));
    mock.cmplx(Some(3));
}

#[test]
#[should_panic(expected="None doesn\\'t satisfy to |t: &Option<u32>| t.is_some()")]
fn test_check_macro_mismatch() {
    let mut scenario = Scenario::new();
    let mock = scenario.create_mock::<A>();
    scenario.expect(mock.cmplx_call(check!(|t:&Option<u32>| t.is_some())).and_return(()));
    mock.cmplx(None);
}