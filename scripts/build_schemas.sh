#!/bin/bash
set -e

projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Generates schemas for contracts in the liquidity_hub
for component in "$projectRootPath"/contracts/liquidity_hub/*/; do
  echo "Generating schemas for $(basename $component)..."
  for contract in "$component"*/; do
    cd $contract && cargo schema
  done
done
