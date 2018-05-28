#!/bin/bash

set -e

(cd mockers && cargo publish --allow-dirty)
(cd mockers_derive && cargo publish --allow-dirty)

