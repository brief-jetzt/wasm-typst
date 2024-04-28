#!/bin/bash

set -euo pipefail

# Build the project
wasm-pack build -d js-src/wasm-pkg
