# Mockers

__Mockers__ is a mocking library (and compiler plugin) for Rust.

It is inspired by [Google Mock].

## Getting Started

First you need to know that the mocking magic is implemented as a compiler plugin, so **nightly Rust is required**. Thus you may want to run

```sh
$ multirust override nightly
```

or

```sh
$ rustup override set nightly
```

if you use [rustup](http://rustup.rs/).

Next, add `mockers` and `mockers_macros` as dependencies to your `Cargo.toml`:

```toml
[dependencies]
mockers_macros = "0.4.6"

[dev-dependencies]
mockers = "0.4.6"
```

Now you are ready to start testing.

## Usage

### Basics

Say we have an `air` crate with some trait and a function using this trait:

```rust
// src/lib.rs
#![crate_name = "air"]

pub trait AirConditioner {
    fn make_hotter(&mut self, by: i16);
    fn make_cooler(&mut self, by: i16);
    fn get_temperature(&self) -> i16;
}

pub fn set_temperature_20(cond: &mut AirConditioner) {
    let t = cond.get_temperature();
    if t < 20 {
        cond.make_hotter(20 + t);
    } else {
        cond.make_cooler(t - 20);
    }
}
```

Import the `mockers` crate and the `mockers_macros` compiler plugin:

```rust
// src/lib.rs

#![feature(plugin, custom_derive)]
#![plugin(mockers_macros)]

#[cfg(test)] extern crate mockers;

…
```

Now derive a `Mock` implementation from the trait:

```rust
#[derive(Mock)]
pub trait AirConditioner {
    …
}
```

Ok, lets start testing:

```rust
// src/lib.rs

…

#[cfg(test)]
mod test {

    use super::*;
    use mockers::Scenario;

    #[test]
    fn test_set_temperature_20() {
        let mut scenario = Scenario::new();
        let mut cond = scenario.create_mock_for::<AirConditioner>();

        scenario.expect(cond.get_temperature_call().and_return(16));
        scenario.expect(cond.make_hotter_call(4).and_return(()));

        set_temperature_20(&mut cond);
    }

}
```

Run test:

```sh
Compiling air v0.1.0 (file:///Users/kriomant/Temp/air)
  Running target/debug/air-b2c5f8b6920cb30a

running 1 test
test test::test_set_temperature_20 ... FAILED

failures:

---- test::test_set_temperature_20 stdout ----
thread 'test::test_set_temperature_20' panicked at '
Unexpected call to `AirConditioner#0.make_hotter`

Here are active expectations for same method call:

Expectation `AirConditioner#0.make_hotter(4)`:
 Arg #0: 36 is not equal to 4
',
note: Run with `RUST_BACKTRACE=1` for a backtrace.


failures:
 test::test_set_temperature_20

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured

error: test failed
```

Seems like we have a problem, which is clearly explained: we expected
that `make_hotter` will be called with value `4` and in fact it
was called with value `36`. We found bug in our function.

Lets examine the test content line by line.

```rust
let mut scenario = Scenario::new();
```

Here we create a `Scenario` instance, which tracks all mock objects
and expectations. When the scenario object is destroyed it checks
that all expectations are satisfied and fails otherwise.

```rust
let mut cond = scenario.create_mock_for::<AirConditioner>();
scenario.expect(cond.get_temperature_call().and_return(16));
scenario.expect(cond.make_hotter_call(4).and_return(()));
```

Here we create a mock object which implements the `AirConditioner` trait and
add expectations. Note that the concrete mock type is not specified. In fact the
`#[derive(Mock)]` clause will generate an `AirConditionerMock` struct, i.e.
it just adds a `Mock` suffix to the trait name. But this is an implementation detail.
Don't rely on it.

In addition to methods from the `AirConditioner` trait, the mock object has a second
set of methods which are named after trait methods, but with an additional
`_call` suffix.

In this case, for example, these are the `get_temperature` method used by the tested code
and the `get_temperature_call` method used by the testing code for creating expectations.

`*_call` methods return "call matcher" objects which are used by the scenario
to find expectations matching the performed call. But it isn't an expectation yet,
because we didn't specify any reaction to this call.

So we call `.and_return(16)` and get an expectation object, which may now be
added to the scenario with `scenario.expect(…)`.

Finally we run the function under test:

```rust
set_temperature_20(&mut cond);
```

### Argument Matchers

Consider the expectation from the previous example:

```rust
cond.make_hotter_call(4).and_return(())
```

`*_call` methods have the same number of arguments as the original method.
In this case we just use a fixed value to verify the call, but expectations are
not limited to that.

For every parameter `arg: T` of the original method, the corresponding `_call` method
has an `arg: M where M: MatchArg<T>` parameter, i.e. it receives a matcher for an
argument of type `T`.

Any type `T` which implements `Eq` automatically implements `MatchArg<T>`.
The arguments get matched by checking for equality with the specified value.

This is why we can pass the value `4` to `make_hotter_call`.

The `matchers` module contains other matchers which may be useful:

  * `ANY` will match any value:
    ```rust
    use mockers::matchers::ANY;
    cond.make_hotter_call(ANY).and_return(());
    ```

  * `lt`, `le`, `eq`, `ne`, `ge`, `gt` will compare the argument with a specified value
    using `<`, `<=`, `==`, `!=`, `>=` and `>` respectively:
    ```rust
    use mockers::matchers::le;
    cond.make_hotter_call(le(5)).and_return(());
    ```

  * `in_range` will check whether the value is contained in range:
    ```rust
    use mockers::matchers::in_range;
    cond.make_hotter_call(in_range(1..)).and_return(());
    cond.make_hotter_call(in_range(10..20)).and_return(());
    ```

  * `not`, `and`, `or` will combine other matchers:
    ```rust
    use mockers::matchers::{gt, lt};
    cond.make_hotter_call(and(gt(3), lt(10))).and_return(());
    ```

  * `none`, `some`, `ok`, `err` matchers for `Option` and `Result`
    ```rust
    use mockers::matchers::{some, lt};
    cond.opt_call(some(gt(3))).and_return(());
    ```

You can also use a function returning `bool` to match an argument:

```rust
use mockers::matchers::check;
cond.make_hotter_call(check(|t: usize| t > 4)).and_return(());
```

While the provided named matchers will produce nice error messages in case
of argument value mismatch, like ```4 is not greater than 5```, checks
using a custom function will produce a non-informative ```<custom function>``` output.

You can improve the error message by using the `check!` macro instead of the `check`
function:

```rust
#[macro_use(check)] extern crate mockers;
cond.make_hotter_call(check!(|t: usize| t > 4)).and_return(());
```

In case of failure it produces: ```3 doesn't satisfy to |t: usize| t > 4```,
which is more useful.

Another useful macro is `arg!`, which allows to check whether an argument
matches a specified pattern:

```rust
#[macro_use(arg)] extern crate mockers;
mock.method_receiving_option_call(arg!(Some(_))).and_return(())
```

It will print something like ```None isn't matched by Some(_)``` in
case of failure.

### Reactions

You already know that we have to add a reaction to a call match to
create an expectation. We have already used the `and_return` reaction, but
there are others:

  * `call_match.and_panic(msg)` will panic with given message;
  * `call_match.and_call(|arg| { arg + 1 })` will call provided closure and
    returns its result;
  * `call_match.and_return_default()` will create and return the default value for types implementing `Default`.

### Expecting no calls

Sometimes you have to ensure that a specified call won't be performed.
You may use the `never()` reaction for this:

```rust
scenario.expect(cond.make_hotter_call(ANY).never());
```

### Expecting several calls

Note that the mock call result is passed to `and_return` by value. Obviously
it may be used just once in common case. This is why specifying such a
reaction creates an expectation which will match just one call.

Same applies when the `and_call`: `FnOnce` closure is used instead.

However, when the result type implements `Clone`, it is possible to return
its copies several times.

Thus, there are additional methods on call matchers: `and_return_clone` and `and_call_clone`.
They are available only when the result type is clonable (or the closure is `FnMut`).

Calling these methods won't return an expectation, because it is not clear
yet, how many times a call must be matched. So you have to additionally call `times`
on it:

```rust
scenario.expect(cond.get_temperature_call().and_return_clone(16).times(2));
```

### Order of calls

The order in which calls are made is not important, expectations are not ordered.
Thus following will succeed:

```rust
scenario.expect(cond.make_hotter_call(4).and_return(()));
scenario.expect(cond.get_temperature_call().and_return(16));

let _temp = cond.get_temperature();
cond.make_hotter(2);
```

If you want to verify that calls are made in specific order, you may
use `Sequence` like this:

```rust
use mockers::Sequence;
…

let mut seq = Sequence::new();
seq.expect(cond.get_temperature_call().and_return(16));
seq.expect(cond.make_hotter_call(4).and_return(()));
scenario.expect(seq);

let _temp = cond.get_temperature();
cond.make_hotter(2);
```

### Matching calls

It is possible that one call matches several expectations:

```rust
scenario.expect(cond.make_hotter_call(ANY).and_panic("boom"));
scenario.expect(cond.make_hotter_call(4).and_return(()));

cond.make_hotter(4);
```

Here `4` matches both `4` and `ANY`. The rule is that the most recent
matching expectation is used. This allows to mock the general
behavior first and then override it for some specific values.

### Checkpoints

Sometimes you want to be sure that at some test point all current
expectations are satisfied and only then specify new ones and continue
testing. You may do this with `checkpoint`.

```rust
scenario.expect(cond.make_hotter_call(4).and_return(()));
cond.make_hotter(4);

scenario.checkpoint();

scenario.expect(cond.make_hotter_call(5).and_return(()));
cond.make_hotter(5);
```

There is an implicit checkpoint call when a scenario object is destroyed.

### Usage from Test Crate

Using `#[derive(Mock)]` is the easiest way to create a mock.

However sometimes you don't want to have tests-related code in your `src` directory. Or a trait you want to mock is from another crate.

(Note that all items produced by `#[derive(Mock)]` are wrapped with #[cfg(test)], so it won't go into your production binary.)

Anyway, this is how you can "mockify" an external trait.

```rust
// tests/lib.rs
#![feature(plugin)]
#![plugin(mockers_macros)]

extern crate mockers;

mock!{
    AirConditionerMock,  // Mock type name
    air, // This is mocked trait's package
    trait AirConditioner {
        fn make_hotter(&mut self, by: i16);
        fn make_cooler(&mut self, by: i16);
        fn get_temperature(&self) -> i16;
    }
}

#[test]
fn test() {
    // Create scenario as usual.
    let mut scenario = Scenario::new();

    // Use `create_mock` with mock type name instead of
    // `create_mock_for` with mocked trait name.
    let mut cond = scenario.create_mock::<AirConditionerMock>();

    // The rest is the same.
    …
}

```

Unfortunately, compiler plugins work on syntax level and
can't infer the trait definition just by its name. So you have
to copy-paste the definition.

### Named mockers

By default, when you create mock objects, they are named
after the mocked trait name and their ordinal number. You may see a mock name in the error message: ```Unexpected call to `AirConditioner#0.make_hotter` ```.

This may be inconvenient when you have several mock objects
of the same type. Just name them!

```rust
let left = scenario.create_named_mock_for::<AirConditioner>("left".to_owned());
let right = scenario.create_named_mock_for::<AirConditioner>("right".to_owned());
```

There is also a corresponding `create_named_mock` method for external trait mocks.

## Error messages

The *Mockers* library tries to produce helpful error messages. It highlights key moments so you can easily spot a problem.
And it provides additional information which may help you to resolve this problem:

![highlighted output](highlight.png)

When no matching expectation is found for a call on some mock object, it will search other mock objects of the same type for matching expectations. This helps to diagnose common problems when an expectation is added for an invalid mock object:

```
error: unexpected call to `AirConditioner#1.get_temperature()`

note: there are no active expectations for AirConditioner#1.get_temperature
note: there are matching expectations for another mock objects

  expectation `AirConditioner#0.get_temperature()`
```

If your test fails and you can't **quickly** understand why, please tell me about your case and we will think how diagnostics can be improved.

[Google Mock]: https://github.com/google/googletest/blob/master/googlemock/README.md
