# What's new

## Dev

### Support for arbitrary self type

Now you can mock methods with arbitrary `self` type, for example
`self: Box<Self>` or `self: Rc<Self>`.

## 0.20.0

### **Breaking change**: introducing mock handles

`Scenario::create_mock*` methods now return tuple `(mock, handle)` instead of
just `mock`. Why? For two reasons:
 * Mocks can now be freely moved, not just passed by reference, because
   you always have "*remote control*" in your hand;
 * Making mock and handle separate objects allows to git rid of `_call` suffix
   on stub methods, they can now be named after original ones.

So, summarizing, before:

```rust
let scenario = Scenario::new();
let mock = scenario.create_mock_for::<dyn A>();

scenario.expect(mock.foo_call().and_return(()));
mock.foo();
```

After:

```rust
let scenario = Scenario::new();
let (mock, handle) = scenario.create_mock_for::<dyn A>();

scenario.expect(handle.foo().and_return(()));
//              ^ `handle.foo`, not `mock.foo_call`
mock.foo();
```

### Simplified `Clone` deriving

Use `derive` attribute option to derive `Clone` implementation instead of
`mock_clone`.

Before:

```rust
#[mocked(AMock)] trait A { .. }
mock_clone!(AMock);

#[mocked(BMock)] trait B { .. }
mock_clone!(BMock, share_expectations);
```

After:
```rust
#[mocked(AMock, derive(Clone))] trait A { .. }
#[mocked(BMock, derive(Clone(share_expectations)))] trait B { .. }
```

### Debugging macro

There is 'debug' feature which causes macros to print generated code to stderr.
Now this code may be formatted using 'rustfmt', just turn on 'debug-rustfmt'
feature.

Additionally, debug print is now turned on on per-attribute basis using 'debug'
attribute option:

```rust
#[mocked(debug)] trait A { .. }
```

Unfortunately, output from all `mock!` invocations is still printed when 'debug'
feature is on, this will be fixed later.

### Fixes for generic traits

Not it is possible to mock traits with both type parameters and associated types.

Deriving `Clone` now works for generic traits.

### Mocking external traits

Traits from another modules or crates may be mocked with `mocked` attribute,
just copy trait definition and use 'extern' option:

```rust
#[mocked(AMock, extern, module="::another::module")]
trait A { .. }
```

Mock name must be provided, because it is not possible to use `create_mock_for`,
only `create_mock`. Module path is mandatory, and it must be absolute (starting
either with '::' or 'crate::').

### Shortcut for `times(1)`

There is now `once()` shortcut for `times(1)`:

```rust
scenario.expect(handle.foo().and_return_clone(1).once());
```

## 0.13.4

`by_ref` matcher allows matching value behind the reference:

```rust
handle.method_with_ref_arg(by_ref(lt(3))).and_return(());
```

## 0.13.2

### Errors reporting

Errors reporting is improved using `proc_macro_diagnostic` feature available on
nightly. Errors can now be attached to positions in source code, so it's
easier to find the cause. Additionally examples are added to some error messages. So
it looks like this:
```
error: Unfortunately, macro can't get full path to referenced parent trait, so it must be be given using 'refs' parameter:

    #[mocked]
    trait A {}

    #[mocked(refs = "A => ::full::path::to::A")]
    trait B : A {}

 --> $DIR/parent_ref.rs:6:11
  |
6 | trait B : A {}
  |           ^
```
