#!/bin/bash

# This script must be synchronized with .travis.yml file

set -e

# `mockers` library itself can be build with any Rust channel
# if you turn off default "nightly" feature.
(cd mockers && cargo +nightly build --no-default-features)
(cd mockers && cargo +stable  build --no-default-features)
# Full-features `mockers` can be build by nightly Rust only.
(cd mockers && cargo +nightly build)
# Tests use `mockers_macros` and thus are only runnable with nightly Rust.
(cd mockers && cargo +nightly test)
# `mockers_macros` and `mockers_derive` use nightly Rust features.
(cd mockers_macros && cargo +nightly build)
(cd mockers_derive && cargo +nightly build)
# `mockers_codegen` can be build with nightly and with stable Rust
# (using `syntex`).
(cd mockers_codegen && cargo +nightly build)
(cd mockers_codegen && cargo +nightly build --features=with-syntex)
(cd mockers_codegen && cargo +stable  build --features=with-syntex)
# Examples
(cd examples/air_macro   && cargo +nightly test)
(cd examples/air_codegen && cargo +nightly test)
(cd examples/air_codegen && cargo +stable  test)

