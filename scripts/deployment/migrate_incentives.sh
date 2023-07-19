#!/usr/bin/env bash

deployment_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

function append_incentive_contract_to_output() {
  if [ $# -eq 3 ]; then
    local incentive=$1
    local pool_label=$2
    local incentive_contract=$3
  else
    echo "append_incentive_contract_to_output needs the incentive, pool_label and incentive_contract"
    exit 1
  fi

  tmpfile=$(mktemp)
  jq --arg incentive_contract $incentive_contract --arg incentive $incentive --arg pool_label $pool_label '.pool_incentives += [{incentive: $incentive, pool_label: $pool_label, incentive_contract: $incentive_contract}]' $output_file >$tmpfile
  mv $tmpfile $output_file
  echo -e "\nStored incentive contract for incentive $incentive on $CHAIN_ID successfully\n"
}

function migrate_incentives() {
  mkdir -p $project_root_path/scripts/deployment/output
  liquidity_hub_output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json

  # get incentive factory address
  incentive_factory_addr=$(jq -r '.contracts[] | select (.wasm == "incentive_factory.wasm") | .contract_address' $liquidity_hub_output_file)
  query='{"incentives":{"limit":30}}'

  incentives=$($BINARY query wasm contract-state smart $incentive_factory_addr "$query" --node $RPC -o json | jq -r '.data[].incentive_address')

  for incentive in $incentives; do
    echo -e "Migrating incentive $incentive"

    MSG='{"migrate_incentive":{"incentive_address": "'$incentive'", "code_id": '$code_id'}}'
    incentive_contract=$($BINARY tx wasm execute $incentive_factory_addr "$MSG" $TXFLAG --from $deployer_address | jq -r '.logs[].events[] | select(.type == "migrate") | .attributes[] | select(.key == "_contract_address") | .value')

    if [ -z "$incentive_contract" ]; then
      echo -e "There was an error migrating incentive $incentive\n"
    fi

    sleep $tx_delay
  done

  # update code id for incentives in the factory
  MSG='{"update_config":{"incentive_code_id": '$code_id'}}'
  $BINARY tx wasm execute $incentive_factory_addr "$MSG" $TXFLAG --from $deployer_address

  echo -e "\n**** Incentives migration successful ****\n"
}

# Displays tool usage
function display_usage() {
  echo "WW Incentive Migrator"
  echo -e "\nUsage:./migrate_incentives.sh [flag]. A single flag should be used, -c to specify the chain.\n"
  echo -e "The script will fetch the incentives from the incentive factory and migrate them with the given code_id.\n"
  echo -e "Available flags:\n"
  echo -e "  -h \thelp"
  echo -e "  -c \tThe chain where you want to migrate incentives (juno|juno-testnet|terra|terra-testnet|... check chain_env.sh for the complete list of supported chains)"
  echo -e "  -i \tThe code_id you want to migrate the incentives to."
}

if [ -z $1 ]; then
  display_usage
  exit 0
fi

# get args
optstring=':c:i:h'
while getopts $optstring arg; do
  source $deployment_script_dir/wallet_importer.sh

  case "$arg" in
  c)
    chain=$OPTARG

    source $deployment_script_dir/deploy_env/chain_env.sh
    init_chain_env $OPTARG
    if [[ "$chain" = "local" ]]; then
      tx_delay=0.5
    else
      tx_delay=8
    fi

    import_deployer_wallet $chain
    ;;
  i)
    code_id=$OPTARG
    migrate_incentives
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
