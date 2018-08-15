
| master | 0.12.2 |
| ------ | ----- |
| [![Build Status](https://travis-ci.org/kriomant/mockers.svg?branch=master)](https://travis-ci.org/kriomant/mockers) [![Coverage Status](https://coveralls.io/repos/github/kriomant/mockers/badge.svg?branch=master)](https://coveralls.io/github/kriomant/mockers?branch=master) | [![Build Status](https://travis-ci.org/kriomant/mockers.svg?branch=0.12.2)](https://travis-ci.org/kriomant/mockers) |



# Mockers

Mocking library for Rust.

Inspired by Google Mock library for C++.

[User Guide]

## Note: breaking change

Versions before 0.10.0 were implemented as compiler plugin and used
`syntex` crate for parsing. Compiler plugins are now deprecated, and
`syntex` crate is not maintained anymore.

Version 0.10.0 migrated from compiler plugin implementation
to usage of `proc_macro_attribute` feature. Since this feature isn't supported
in stable Rust yet, `mockers` is only available on unstable at the moment.
It is possible that support for stable Rust will be implemented even before
`proc_macro_attribute` feature stabilization.

In trivial cases migration of your tests to new `mockers` version is as
simple as replacing `#[derive(Mock)]` with `#[mocked]`.

In version 0.12.0 macro attribute was renamed from "derive_mock" to
"mocked". "derive_mock" attribute is still available and is alias to "mocked",
but it will eventually be deprecated.

## Limitations

For now it is not a full-featured mocking library, but just
a prototype to gather feedback. For example, only methods with
four or fewer arguments are supported, non-'static lifetimes are not
supported and so on.

Features are added on demand, so feel free to contact me and explain your
needs.

Mocking magic is implemented using `proc_macro_attribute` attribute
which is only available on nightly Rust (it was tested to work with
*1.30.0-nightly (67390c0c3 2018-08-14)*). Working on stable Rust
will be supported later.

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
mockers = "0.12.2"
mockers_derive = "0.12.2"
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
      let mut cond = scenario.create_mock_for::<AirConditioner>();

      scenario.expect(cond.get_temperature_call().and_return(16));
      scenario.expect(cond.make_hotter_call(4).and_return(()));

      set_temperature_20(&mut cond);
  }
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
