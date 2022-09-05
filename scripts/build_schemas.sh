#!/bin/bash
set -e

projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Generates schemas for contracts in the liquidity_hub
for component in "$projectRootPath"/contracts/liquidity_hub/*/; do
  echo "Generating schemas for $(basename $component)..."
  if [[ "$(basename $component)" == "fee_collector" ]]; then
    cd $component && cargo schema --locked
  else
    for contract in "$component"*/; do
      cd $contract && cargo schema --locked
    done
  fi
done
