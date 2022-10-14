#!/bin/bash
set -e
# First argument, whether or not to run git diff and exit with an error on any json file diff, not used by default
if [[ -z $1 ]]; then
  fail_diff_flag=false
else
  fail_diff_flag=$1
fi


projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Generates schemas for contracts in the liquidity_hub
for component in "$projectRootPath"/contracts/liquidity_hub/*/; do
  echo "Generating schemas for $(basename $component)..."
  if [[ "$(basename $component)" == "fee_collector" ]]; then
    cd $component && cargo schema --locked
  else
    for contract in "$component"*/; do
      cd $contract && cargo schema --locked

      # Optionally fail on any unaccounted changes in json schema files 
      if [[ "$fail_diff_flag" == true ]]; then
        git diff  --exit-code -- '*.json'
      fi
    done
  fi
done
