
| master | 0.4.6 |
| ------ | ----- |
| [![Build Status](https://travis-ci.org/kriomant/mockers.svg?branch=master)](https://travis-ci.org/kriomant/mockers) [![Coverage Status](https://coveralls.io/repos/github/kriomant/mockers/badge.svg?branch=master)](https://coveralls.io/github/kriomant/mockers?branch=master) | [![Build Status](https://travis-ci.org/kriomant/mockers.svg?branch=0.4.6)](https://travis-ci.org/kriomant/mockers) |



# Mockers

Mocking library for Rust.

Inspired by Google Mock library for C++.

[User Guide]

## Limitations

For now it is not a full-featured mocking library, but just
a prototype to gather feedback. For example, only methods with
four or less arguments are supported, non-'static lifetimes are not
supported and so on.

Mocking magic is implemented using compiler plugin, so **nightly Rust
is required**. It was tested to work with *1.11.0-nightly (1ab87b65a 2016-07-02)*.

## Usage at a glance

This is a very short introduction to show what is possible and
how it looks. Read [User Guide] for details.

This library requires nightly Rust.  

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
[dependencies]
mockers_macros = "0.4.6"

[dev-dependencies]
mockers = "0.4.6"
```

src/lib.rs:

```rust
#![feature(plugin, custom_derive)]
#![plugin(mockers_macros)]

#[cfg(test)] extern crate mockers;

#[derive(Mock)]
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
      let mut scenario = Scenario::new();
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
