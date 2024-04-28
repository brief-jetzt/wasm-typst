#!/bin/bash

set -euo pipefail

# Build the project
wasm-pack build "$@"

# Modify package.json to fit our needs
# https://github.com/rustwasm/wasm-pack/issues/427#issuecomment-458180179

PACKAGE_JSON=$(cat pkg/package.json)

# Namespace the package to be able to publish to GitHub Packages / brief-jetzt organization
PACKAGE_JSON=$(echo "$PACKAGE_JSON" | jq '.["name"] = "@brief-jetzt/wasm-typst"')

# Adjust the publishConfig
PACKAGE_JSON=$(echo "$PACKAGE_JSON" | jq '.["publishConfig"] = {"access": "public"}')


echo "$PACKAGE_JSON" | jq
echo "$PACKAGE_JSON" > pkg/package.json