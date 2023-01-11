#!/bin/bash
set -e

# Import the deploy_liquidity_hub script
deployment_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Displays tool usage
function display_usage() {
  echo "WW Pool Deployer"
  echo -e "\nUsage:./deploy_pool.sh [flags].\n"
  echo -e "Available flags:\n"
  echo -e "  -h \thelp"
  echo -e "  -c \tThe chain where you want to deploy (juno|juno-testnet|terra|terra-testnet|... check chain_env.sh for the complete list of supported chains)"
  echo -e "  -p \tPool configuration file to get deployment info from."
}

# Reads a pool config file, like the follow:
#
#{
#  "protocol_fee": "0.001",
#  "swap_fee": "0.002",
#  "burn_fee": "0.002",
#  "assets": [
#    {
#      "asset": "uluna",
#      "is_native": true
#    },
#    {
#      "asset": "terra1rzvdn9cc7efpqsgl4ha7q5egqlpnyfdkgu0at6fkzmmj9dr7aspsy4a5js",
#      "is_native": false
#    }
#  ]
#}
function read_pool_config() {
  if [ $# -eq 1 ]; then
    local pool=$1
  else
    echo "read_pool_config requires a pool config file"
    exit 1
  fi

  mapfile -t assets < <(jq -c '.assets[]' <$pool)
  protocol_fee=$(jq -r '.protocol_fee' $pool)
  swap_fee=$(jq -r '.swap_fee' $pool)
  burn_fee=$(jq -r '.burn_fee' $pool)
}

function check_decimals() {
  if [ $# -eq 2 ]; then
    local denom=$1
    local decimals=$2
  else
    echo "check_decimals requires the denom and the decimals"
    exit 1
  fi

  query='{"native_token_decimals":{"denom":"'$denom'"}}'
  local res=$($BINARY query wasm contract-state smart $pool_factory_addr "$query" --node $RPC --output json | jq -r '.data.decimals')
  if [[ -z "$res" ]]; then
    echo "Adding native decimals to factory..."
    # the factory doesn't have the decimals for this denom, registration needs to happen before creating the pool
    add_native_decimals_msg='{"add_native_token_decimals":{"denom":"'$denom'","decimals":'$decimals'}}'

    local res=$($BINARY tx wasm execute $pool_factory_addr "$add_native_decimals_msg" $TXFLAG --amount 1$DENOM --from $deployer_address)
    echo $res
    sleep $tx_delay
  fi
}

function create_pool() {
  mkdir -p $project_root_path/scripts/deployment/output
  output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_pools.json
  deployment_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json

  if [[ ! -f "$output_file" ]]; then
    # create file to dump results into
    echo '{"pools": []}' | jq '.' >$output_file
  fi

  pool_factory_addr=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $deployment_file)

  for asset in "${assets[@]}"; do
    is_native=$(echo $asset | jq '.is_native')

    if [[ $is_native == "true" ]]; then
      check_decimals $(echo $asset | jq -r '.asset') $(echo $asset | jq -r '.decimals')
      asset_info='{"native_token":{"denom":"'$(echo $asset | jq -r '.asset')'"}}'
    else
      asset_info='{"token":{"contract_addr":"'$(echo $asset | jq -r '.asset')'"}}'
    fi
    asset_infos+=($asset_info)
  done

  create_pool_msg='{"create_pair":{"asset_infos":['${asset_infos[0]}','${asset_infos[1]}'],"pool_fees":{"protocol_fee":{"share":"'$protocol_fee'"},"burn_fee":{"share":"'$burn_fee'"},"swap_fee":{"share":"'$swap_fee'"}}}}'

  echo "Creating pool with the following configuration:"
  echo "Asset 0: ${asset_infos[0]}"
  echo "Asset 1: ${asset_infos[1]}"
  echo "Protocol fee: $protocol_fee"
  echo -e "Swap fee: $swap_fee"
  echo -e "Burn fee: $burn_fee\n"

  local res=$($BINARY tx wasm execute $pool_factory_addr "$create_pool_msg" $TXFLAG --from $deployer_address)
  echo $res

  local pair=$(echo $res | jq -r '.logs[0].events[] | select(.type == "wasm").attributes[] | select(.key == "pair").value')
  local pool_address=$(echo $res | jq -r '.logs[0].events[] | select(.type == "wasm").attributes[] | select(.key == "pair_contract_addr").value')
  local lp_address=($(echo $res | jq -r '.logs[0].events[] | select(.type == "wasm").attributes[] | select(.key == "liquidity_token_addr").value'))
  local code_ids=($(echo $res | jq -r '.logs[0].events[] | select(.type == "instantiate").attributes[] | select(.key == "code_id").value'))

  # Store on output file
  tmpfile=$(mktemp)
  jq -r --arg pair $pair --arg asset0 ${asset_infos[0]} --arg asset1 ${asset_infos[1]} --arg pool_address $pool_address --arg lp_address ${lp_address[0]} --arg pool_code_id ${code_ids[0]} --arg lp_code_id ${code_ids[1]} '.pools += [{pair: $pair, assets: [$asset0, $asset1], pool_address: $pool_address, lp_address: $lp_address, pool_code_id: $pool_code_id, lp_code_id: $lp_code_id }]' $output_file >$tmpfile
  mv $tmpfile $output_file

  # Add additional deployment information
  date=$(date -u +"%Y-%m-%dT%H:%M:%S%z")
  tmpfile=$(mktemp)
  jq --arg date $date --arg chain_id $CHAIN_ID --arg pool_factory_addr $pool_factory_addr '. + {date: $date , chain_id: $chain_id, pool_factory_addr: $pool_factory_addr}' $output_file >$tmpfile
  mv $tmpfile $output_file

  echo -e "\n**** Created ${asset_infos[0]}-${asset_infos[1]} pool on $CHAIN_ID successfully ****\n"
  jq '.' $output_file
}

if [ -z $1 ]; then
  display_usage
  exit 0
fi

# get args
optstring=':c:p:h'
while getopts $optstring arg; do
  case "$arg" in
  c)
    chain=$OPTARG
    source $deployment_script_dir/deploy_env/chain_env.sh
    init_chain_env $OPTARG
    if [[ "$chain" = "local" ]]; then
      tx_delay=0.5s
    else
      tx_delay=8s
    fi
    ;;
  p)
    source $deployment_script_dir/wallet_importer.sh
    import_deployer_wallet $chain

    # read pool config from file $OPTARG
    read_pool_config $OPTARG && create_pool
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
