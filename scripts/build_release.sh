#!/bin/bash
set -e

projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# if the operative system is running arm64, append -arm64 to workspace-optimizer. Otherwise not
arch=$(uname -m)

docker_options=(
  --rm
  -v "$projectRootPath":/code
  --mount type=volume,source="$(basename "$projectRootPath")_cache",target=/target
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry
)

# Optimized builds
if [[ "$arch" == "aarch64" || "$arch" == "arm64" ]]; then
  docker run "${docker_options[@]}" cosmwasm/workspace-optimizer-arm64:0.13.0
else
  docker run "${docker_options[@]}" cosmwasm/workspace-optimizer:0.13.0
fi

# Check generated wasm file sizes
$projectRootPath/scripts/check_artifacts_size.sh
