#!/bin/bash

set -euo pipefail

# Build the project
wasm-pack build "$@"

# Modify package.json to fit our needs
# https://github.com/rustwasm/wasm-pack/issues/427#issuecomment-458180179

PACKAGE_JSON=$(cat pkg/package.json)
PACKAGE_JSON=$(echo "$PACKAGE_JSON" | jq '.["name"] = "@brief-jetzt/wasm-typst"')
PACKAGE_JSON=$(echo "$PACKAGE_JSON" | jq '.["publishConfig"] = {"access": "public"}')
PACKAGE_JSON=$(echo "$PACKAGE_JSON" | jq '.["repository"] = {"type": "git", "url": "https://github.com/brief-jetzt/wasm-typst"}')


echo "$PACKAGE_JSON" | jq
echo "$PACKAGE_JSON" > pkg/package.json