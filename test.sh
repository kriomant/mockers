#!/bin/bash

# This script must be synchronized with .travis.yml file

set -e

# `mockers` library itself can be build with any Rust channel
# if you turn off default "nightly" feature.
(echo Build mockers/nightly/no-default-features ; cd mockers && cargo +nightly build --no-default-features)
(echo Build mockers/stable/no-default-features ; cd mockers && cargo +stable  build --no-default-features)
# Full-features `mockers` can be build by nightly Rust only.
(echo mockers/nighly ; cd mockers && cargo +nightly build)
# Tests use `mockers_macros` and thus are only runnable with nightly Rust.
(echo mockers/nightly/test ; cd mockers && cargo +nightly test)
# `mockers_macros` and `mockers_derive` use nightly Rust features.
(echo mockers_macros/nightly ; cd mockers_macros && cargo +nightly build)
(echo mockers_derive/nightly ; cd mockers_derive && cargo +nightly build)
# `mockers_codegen` can be build with nightly and with stable Rust
# (using `syntex`).
(echo mockers_codegen/nightly ; cd mockers_codegen && cargo +nightly build)
(echo mockers_codegen/nightly/with-syntex ; cd mockers_codegen && cargo +nightly build --features=with-syntex)
(echo mockers_codegen/stable/with-syntex; cd mockers_codegen && cargo +stable  build --features=with-syntex)
# Examples
(echo air_macro/nightly/test ; cd examples/air_macro   && cargo +nightly test)
(echo air_codegen/nightly/test ; cd examples/air_codegen && cargo +nightly test)
(echo air_codegen/stable/test ; cd examples/air_codegen && cargo +stable  test)

