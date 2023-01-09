#!/bin/bash
set -e

project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Imports a wallet to deploy
function import_deployer_wallet() {
  if [ $# -eq 1 ]; then
    local chain=$1
  else
    echo "import_deployer_wallet requires a chain to load the right mnemonic"
    exit 1
  fi

  if [[ "$(echo ${chain##*-})" = "testnet" ]] || [[ "$chain" = "local" ]]; then
    deployer='deployer_wallet_testnet'
    local mnemonic=$(cat "$project_root_path"/scripts/deployment/deploy_env/mnemonics/deployer_mnemonic_testnet.txt)
  else
    deployer='deployer_wallet'
    local mnemonic=$(cat "$project_root_path"/scripts/deployment/deploy_env/mnemonics/deployer_mnemonic.txt)
  fi

  # verify if the deployer wallet has already been imported
  if ! $BINARY keys show $deployer >/dev/null 2>&1; then
    # wallet needs to be imported
    echo "Importing $deployer into $BINARY..."
    echo $mnemonic | $BINARY keys add $deployer --recover >/dev/null 2>&1
  fi

  deployer_address=$($BINARY keys show $deployer --output json | jq -r '.address')
}
