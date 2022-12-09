#!/bin/bash
set -e

deployment_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)
tx_delay=8s

# Displays tool usage
function display_usage() {
  echo "WW Liquidity Hub Migrator"
  echo -e "\nUsage:./migrate_liquidity_hub.sh [flags]. Two flags should be used, -c to specify the chain and -m.\n"
  echo -e "Available flags:\n"
  echo -e "  -h \thelp"
  echo -e "  -c \tThe chain where you want to migrate (juno|juno-testnet|terra|terra-testnet)"
  echo -e "  -m \tWhat to migrate (all|pool-network|vault-network|fee-collector|pool-factory|pool|token|pool-router|vaults|vault-factory|vault-router)"
}

function migrate_fee_collector() {
  migrate_msg='{}'
  migrate_artifact $project_root_path/artifacts/fee_collector.wasm $migrate_msg
}

function migrate_pool_factory() {
  migrate_msg='{}'
  migrate_artifact $project_root_path/artifacts/terraswap_factory.wasm $migrate_msg
}

function migrate_pool_router() {
  migrate_msg='{}'
  migrate_artifact $project_root_path/artifacts/terraswap_router.wasm $migrate_msg
}

function migrate_pools() {
  echo -e "\nMigrating Pools on $CHAIN_ID..."

  code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_pair.wasm") | .code_id' $output_file)
  factory_address=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $output_file)
  query='{"pairs":{"limit":30}}'

  pools=$($BINARY query wasm contract-state smart $factory_address "$query" --node $RPC -o json | jq -r '.data.pairs[].contract_addr')
  for pool in $pools; do
    echo -e "Migrating $pool"

    MSG='{"migrate_pair":{"contract":"'$pool'","code_id": '$code_id'}}'
    $BINARY tx wasm execute $factory_address "$MSG" $TXFLAG --from $deployer
    sleep $tx_delay
  done

  # Update the code_id on the factory
  MSG='{"update_config":{"pair_code_id":'$code_id'}}'
  $BINARY tx wasm execute $factory_address "$MSG" $TXFLAG --from $deployer
  sleep $tx_delay
}

function migrate_vault_factory() {
  migrate_msg='{}'
  migrate_artifact $project_root_path/artifacts/vault_factory.wasm $migrate_msg
}

function migrate_vault_router() {
  migrate_msg='{}'
  migrate_artifact $project_root_path/artifacts/vault_router.wasm "$migrate_msg"
}

function migrate_vaults() {
  echo -e "\nMigrating Vaults on $CHAIN_ID..."

  code_id=$(jq -r '.contracts[] | select (.wasm == "vault.wasm") | .code_id' $output_file)
  factory_address=$(jq -r '.contracts[] | select (.wasm == "vault_factory.wasm") | .contract_address' $output_file)
  query='{"vaults":{"limit":30}}'

  vaults=$($BINARY query wasm contract-state smart $factory_address "$query" --node $RPC -o json | jq -r '.data.vaults[].vault')
  for vault in $vaults; do
    echo -e "Migrating $vault"

    MSG='{"migrate_vaults":{"vault_addr":"'$vault'","vault_code_id": '$code_id'}}'
    $BINARY tx wasm execute $factory_address "$MSG" $TXFLAG --from $deployer
    sleep $tx_delay
  done

  # Update the code_id on the factory
  MSG='{"update_config":{"vault_id":'$code_id'}}'
  $BINARY tx wasm execute $factory_address "$MSG" $TXFLAG --from $deployer
  sleep $tx_delay
}

function migrate_pool_network() {
  migrate_fee_collector
  migrate_pool_factory
  migrate_pool_router
  migrate_pools
}

function migrate_vault_network() {
  migrate_vault_factory
  migrate_vault_router
  migrate_vaults
}

function migrate_liquidity_hub() {
  echo -e "\nMigrating the Liquidity Hub on $CHAIN_ID..."
  migrate_pool_network
  migrate_vault_network
}

function migrate_artifact() {
  if [ $# -eq 2 ]; then
    local artifact=$1
    local migrate_msg=$2
  else
    echo "migrate_artifact needs the artifact and migrate_msg"
    exit 1
  fi

  echo "Migrating $(basename $artifact) on $CHAIN_ID..."

  contract_address=$(jq -r '.contracts[] | select (.wasm == "'$(basename $artifact)'") | .contract_address' $output_file)
  code_id=$(jq -r '.contracts[] | select (.wasm == "'$(basename $artifact)'") | .code_id' $output_file)

  $BINARY tx wasm migrate $contract_address $code_id "$migrate_msg" $TXFLAG --from $deployer
  sleep $tx_delay
}

function migrate() {
  mkdir -p $project_root_path/scripts/deployment/output
  output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json

  if [[ ! -f "$output_file" ]]; then
    echo "$output_file does not exist. Please run deploy_liquidity_hub.sh first."
    exit 1
  fi

  echo "This script assumes the code_ids in $output_file are the ones to be migrated to. If that is not the case, please
   run deploy_liquidity_hub.sh using the option -s to store the new artifacts on chain and update the code_ids in $output_file."
  echo -e "\nThe migration will take the code_ids from $output_file. Do you want to proceed? (y/n)"
  read proceed

  if [[ "$proceed" != "y" ]]; then
    echo "Migrated cancelled..."
    exit 1
  fi

  case $1 in
  pool-network)
    migrate_pool_network
    ;;
  vault-network)
    migrate_vault_network
    ;;
  fee-collector)
    migrate_fee_collector
    ;;
  pool-factory)
    migrate_pool_factory
    ;;
  pools)
    migrate_pools
    ;;
  pool-router)
    migrate_pool_router
    ;;
  vaults)
    migrate_vaults
    ;;
  vault-factory)
    migrate_vault_factory
    ;;
  vault-router)
    migrate_vault_router
    ;;
  *) # migrate all
    migrate_liquidity_hub
    ;;
  esac

  echo -e "\n**** Migration successful ****\n"
}

if [ -z $1 ]; then
  display_usage
  exit 0
fi

# get args
optstring=':c:m:h'
while getopts $optstring arg; do
  source $deployment_script_dir/wallet_importer.sh

  case "$arg" in
  c)
    chain=$OPTARG
    source $deployment_script_dir/deploy_env/chain_env.sh
    init_chain_env $OPTARG
    ;;
  m)
    import_deployer_wallet $chain
    migrate $OPTARG
    ;;
  h)
    display_usage
    exit 0
    ;;
  :)
    echo "Must supply an argument to -$OPTARG" >&2
    exit 1
    ;;
  ?)
    echo "Invalid option: -${OPTARG}"
    display_usage
    exit 2
    ;;
  esac
done
