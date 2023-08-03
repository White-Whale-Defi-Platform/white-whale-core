#!/usr/bin/env bash

deployment_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

function append_incentive_contract_to_output() {
  if [ $# -eq 3 ]; then
    local pool=$1
    local pool_label=$2
    local incentive_contract=$3
  else
    echo "append_incentive_contract_to_output needs the pool, pool_label and incentive_contract"
    exit 1
  fi

  tmpfile=$(mktemp)
  jq --arg incentive_contract $incentive_contract --arg pool $pool --arg pool_label $pool_label '.pool_incentives += [{pool: $pool, pool_label: $pool_label, incentive_contract: $incentive_contract}]' $output_file >$tmpfile
  mv $tmpfile $output_file
  echo -e "\nStored incentive contract for pool $pool on $CHAIN_ID successfully\n"
}

function create_incentives() {
  mkdir -p $project_root_path/scripts/deployment/output
  liquidity_hub_output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json
  output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_pool_incentives.json

  if [[ ! -f "$output_file" ]]; then
    # create file to dump results into
    echo '{"pool_incentives": []}' | jq '.' >$output_file
    initial_block_height=$(curl -s $RPC/abci_info? | jq -r '.result.response.last_block_height')
  else
    # read from existing deployment file
    initial_block_height=$(jq -r '.initial_block_height' $output_file)
  fi

  # get pool factory address
  pool_factory_addr=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $liquidity_hub_output_file)
  query='{"pairs":{"limit":30}}'

  pools=$($BINARY query wasm contract-state smart $pool_factory_addr "$query" --node $RPC -o json | jq -r '.data.pairs[].contract_addr')
  for pool in $pools; do
    echo -e "Creating incentive contract for $pool"
    query='{"pair":{}}'

    pool_label=$($BINARY q wasm contract $pool --node $RPC -o json | jq -r '.contract_info.label')
    lp_asset_info=$($BINARY query wasm contract-state smart $pool "$query" --node $RPC -o json | jq -r '.data.liquidity_token | tostring')

    incentive_factory_addr=$(jq -r '.contracts[] | select (.wasm == "incentive_factory.wasm") | .contract_address' $liquidity_hub_output_file)
    MSG='{"create_incentive":{"lp_asset":'$lp_asset_info'}}'

    incentive_contract=$($BINARY tx wasm execute $incentive_factory_addr "$MSG" $TXFLAG --from $deployer_address | jq -r '.logs[].events[] | select(.type == "instantiate") | .attributes[] | select(.key == "_contract_address") | .value')

    if [ -n "$incentive_contract" ]; then
      # Append incentive_contract to output file
      append_incentive_contract_to_output $pool $(echo "$pool_label" | sed 's/ pair//') $incentive_contract
    fi

    sleep $tx_delay
  done

  final_block_height=$(curl -s $RPC/abci_info? | jq -r '.result.response.last_block_height')

  # Add additional deployment information
  date=$(date -u +"%Y-%m-%dT%H:%M:%S%z")
  tmpfile=$(mktemp)
  jq --arg date $date --arg chain_id $CHAIN_ID --arg deployer_address $deployer_address --arg initial_block_height $initial_block_height --arg final_block_height $final_block_height '. + {date: $date , initial_block_height: $initial_block_height, final_block_height: $final_block_height, chain_id: $chain_id, deployer_address: $deployer_address}' $output_file >$tmpfile
  mv $tmpfile $output_file

  echo -e "\n**** Incentives creation successful ****\n"
  jq '.' $output_file
}

# Displays tool usage
function display_usage() {
  echo "WW Incentive Creator"
  echo -e "\nUsage:./create_incentives.sh [flag]. A single flag should be used, -c to specify the chain.\n"
  echo -e "The script will fetch the pools from the pool factory and create incentives with their LP tokens, providing an output with all the details once it's done.\n"
  echo -e "Available flags:\n"
  echo -e "  -h \thelp"
  echo -e "  -c \tThe chain where you want to create incentives (juno|juno-testnet|terra|terra-testnet|... check chain_env.sh for the complete list of supported chains)"
}

# get args
optstring=':c:h'
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
    create_incentives
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
