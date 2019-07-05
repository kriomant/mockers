
| master | 0.21.0 |
| ------ | ----- |
| [![Build Status](https://travis-ci.org/kriomant/mockers.svg?branch=master)](https://travis-ci.org/kriomant/mockers) [![Coverage Status](https://coveralls.io/repos/github/kriomant/mockers/badge.svg?branch=master)](https://coveralls.io/github/kriomant/mockers?branch=master) | [![Build Status](https://travis-ci.org/kriomant/mockers.svg?branch=0.21.0)](https://travis-ci.org/kriomant/mockers) |



# Mockers

Mocking library for Rust. Both stable and nightly Rust are supported.

Inspired by Google Mock library for C++.

**Warning**: There are breaking changes in 0.21.0 release, read What's New for details.

[What's New]
[User Guide]

## Limitations 
For now it is not a full-featured mocking library, but just
a prototype to gather feedback. For example, only methods with
four or fewer arguments are supported, non-'static lifetimes are not
supported and so on.

Features are added on demand, so feel free to contact me and explain your
needs.

## Usage at a glance

This is a very short introduction to show what is possible and
how it looks. Read [User Guide] for details.

We will use nightly Rust in this example for simplicity.

For multirust, run the following command:
```
$ multirust override nightly
```

Or if you're using rustup:

```
$ rustup override set nightly
```

Cargo.toml:

```toml
[dev-dependencies]
mockers = "0.21.0"
mockers_derive = "0.21.0"
```

src/lib.rs:

```rust
#![feature(use_extern_macros)]

#[cfg(test)] extern crate mockers_derive;

#[cfg(test)] use mockers_derive::mocked;

#[cfg_attr(test, mocked)]
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

#[cfg(test)]
mod test {
  use super::*;
  use mockers::Scenario;

  #[test]
  fn test_set_temperature_20() {
      let scenario = Scenario::new();
      let (mut cond, cond_handle) = scenario.create_mock_for::<AirConditioner>();

      scenario.expect(cond_handle.get_temperature().and_return(16));
      scenario.expect(cond_handle.make_hotter(4).and_return(()));

      set_temperature_20(&mut cond);
  }
}
```

And if you're using `2018` edition:

```rust
#[cfg(test)] use mockers_derive::mocked;

#[cfg_attr(test, mocked)]
pub trait AirConditioner {
    fn make_hotter(&mut self, by: i16);
    fn make_cooler(&mut self, by: i16);
    fn get_temperature(&self) -> i16;
}
```

Run tests:

```
$ cargo test
   Compiling air v0.1.0 (file:///Users/kriomant/Temp/air)
     Running target/debug/air-b2c5f8b6920cb30a

running 1 test
test test::test_set_temperature_20 ... FAILED

failures:

---- test::test_set_temperature_20 stdout ----
	thread 'test::test_set_temperature_20' panicked at '

error: unexpected call to `AirConditioner#0.make_hotter(36)`

note: here are active expectations for AirConditioner#0.make_hotter

  expectation `AirConditioner#0.make_hotter(4)`:
    arg #0: 36 is not equal to 4

'
note: Run with `RUST_BACKTRACE=1` for a backtrace.


failures:
    test::test_set_temperature_20

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured

error: test failed
```

## License

Copyright © 2016 Mikhail Trishchenkov

Distributed under the [MIT License](LICENSE).

[User Guide]: doc/guide.md
[What's New]: doc/whats-new.md

