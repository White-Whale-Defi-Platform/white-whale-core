#!/bin/bash
set -e

projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Optimized builds
docker run --rm -v "$projectRootPath":/code \
  --mount type=volume,source="$(basename "$projectRootPath")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.10

# Check generated wasm file sizes
$projectRootPath/scripts/check_artifacts_size.sh
