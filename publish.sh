#!/bin/bash

set -e

(cd mockers_codegen && cargo publish)
(cd mockers_macros && cargo publish)
(cd mockers && cargo publish)

