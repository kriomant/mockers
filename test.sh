#!/bin/bash

# This script must be synchronized with .travis.yml file

set -e

(echo mockers/stable ; cd mockers && cargo +stable build)
(echo mockers/stable/test ; cd mockers && cargo +stable test)
(echo mockers_derive/stable ; cd mockers_derive && cargo +stable build)
# `mockers` has more features on nightly Rust.
(echo mockers/nightly ; cd mockers && cargo +nightly build --features nightly)
(echo mockers/nightly/test ; cd mockers && cargo +nightly test --features nightly)
(echo mockers_derive/nightly ; cd mockers_derive && cargo +nightly build --features nightly)
(echo mockers_derive/nightly/test ; cd mockers_derive && cargo +nightly test --features nightly)
# Examples
(echo air_proc_macro/stable/test ; cd examples/air_proc_macro && cargo +stable test)
(echo air_proc_macro/nightly/test ; cd examples/air_proc_macro && cargo +nightly test --features nightly)
