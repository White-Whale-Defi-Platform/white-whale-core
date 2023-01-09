#!/bin/bash
set -e

# This script creates a new local wasmd chain for local testing.

chain=local

deployment_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
source $deployment_script_dir/deploy_env/chain_env.sh
init_chain_env $chain

rm -rf ~/.$BINARY
$BINARY init testnode --chain-id=$CHAIN_ID

config_toml=~/.$BINARY/config/config.toml

sed -i 's/timeout_propose = ".*"/timeout_propose = "100ms"/' $config_toml
sed -i 's/timeout_propose_delta = ".*"/timeout_propose_delta = "100ms"/' $config_toml
sed -i 's/timeout_prevote = ".*"/timeout_prevote = "100ms"/' $config_toml
sed -i 's/timeout_prevote_delta = ".*"/timeout_prevote_delta = "100ms"/' $config_toml
sed -i 's/timeout_precommit = ".*"/timeout_precommit = "100ms"/' $config_toml
sed -i 's/timeout_precommit_delta = ".*"/timeout_precommit_delta = "100ms"/' $config_toml
sed -i 's/timeout_commit = ".*"/timeout_commit = "100ms"/' $config_toml

source $deployment_script_dir/wallet_importer.sh
import_deployer_wallet $chain

$BINARY add-genesis-account deployer_wallet_testnet 100000000000000000000stake
$BINARY gentx deployer_wallet_testnet 1000000stake --chain-id $CHAIN_ID
$BINARY collect-gentxs

$BINARY start
