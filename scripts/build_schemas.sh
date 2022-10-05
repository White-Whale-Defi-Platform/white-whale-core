#!/bin/bash
set -e
# First argument, whether or not to run git diff and exit with an error on any json file diff, not used by default
ARG1=${1:-0}
projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Generates schemas for contracts in the liquidity_hub
for component in "$projectRootPath"/contracts/liquidity_hub/*/; do
  echo "Generating schemas for $(basename $component)..."
  if [[ "$(basename $component)" == "fee-collector" ]]; then
    cd $component && cargo schema
  else
    for contract in "$component"*/; do
      cd $contract && cargo schema

      # Optionally fail on any unaccounted changes in json schema files 
      if [[ ARG1 ]]; then
        git diff  --exit-code -- '*.json'
      fi
    done
  fi
done
