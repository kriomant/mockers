#!/bin/bash

# This script must be synchronized with .travis.yml file

set -e

(echo mockers/stable ; cd mockers && cargo +stable build)
# `mockers` has more features on nightly Rust.
(echo mockers/nightly ; cd mockers && cargo +nightly build --features nightly)
# Tests use `mockers_derive` and thus are only runnable with nightly Rust.
(echo mockers/nightly/test ; cd mockers && cargo +nightly test --features nightly)
# `mockers_derive` uses nightly Rust features.
(echo mockers_derive/nightly ; cd mockers_derive && cargo +nightly build)
# Examples
(echo air_proc_macro/nightly/test ; cd examples/air_proc_macro && cargo +nightly test)
