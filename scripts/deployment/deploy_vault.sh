#!/bin/bash
set -e

# Import the deploy_liquidity_hub script
deployment_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Displays tool usage
function display_usage() {
  echo "WW Vault Deployer"
  echo -e "\nUsage:./deploy_vault.sh [flags].\n"
  echo -e "Available flags:\n"
  echo -e "  -h \thelp"
  echo -e "  -c \tThe chain where you want to deploy (juno|juno-testnet|terra|terra-testnet|... check chain_env.sh for the complete list of supported chains)"
  echo -e "  -v \tVault configuration file to get deployment info from."
}

# Reads a vault config file, like the follow:
#
# {
#  "asset": "ujuno", //or contract
#  "protocol_fee": "0.01",
#  "flash_loan_fee": "0.02",
#  "burn_fee": "0.02",
#  "is_native": true //or false
# }
#
function read_vault_config() {
  if [ $# -eq 1 ]; then
    local vault=$1
  else
    echo "read_vault_config requires a vault config file"
    exit 1
  fi

  asset=$(jq -r '.asset' $vault)
  protocol_fee=$(jq -r '.protocol_fee' $vault)
  flash_loan_fee=$(jq -r '.flash_loan_fee' $vault)
  burn_fee=$(jq -r '.burn_fee' $vault)
  is_native=$(jq -r '.is_native' $vault)
}

function create_vault() {
  mkdir -p $project_root_path/scripts/deployment/output
  output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_vaults.json
  deployment_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json

  if [[ ! -f "$output_file" ]]; then
    # create file to dump results into
    echo '{"vaults": []}' | jq '.' >$output_file
  fi

  vault_factory_addr=$(jq -r '.contracts[] | select (.wasm == "vault_factory.wasm") | .contract_address' $deployment_file)

  if [[ $is_native == "true" ]]; then
    asset_info='{"native_token":{"denom":"'$asset'"}}'
  else
    asset_info='{"token":{"contract_addr":"'$asset'"}}'
  fi

  create_vault_msg='{"create_vault":{"asset_info":'$asset_info',"fees":{"protocol_fee":{"share":"'$protocol_fee'"},"burn_fee":{"share":"'$burn_fee'"},"flash_loan_fee":{"share":"'$flash_loan_fee'"}}}}'

  echo "Creating vault with the following configuration:"
  echo "Asset: $asset"
  echo "Protocol fee: $protocol_fee"
  echo -e "Flash loan fee: $flash_loan_fee"
  echo -e "Burn fee: $burn_fee\n"

  local res=$($BINARY tx wasm execute $vault_factory_addr "$create_vault_msg" $TXFLAG --from $deployer_address)
  echo $res

  local vault_address=$(echo $res | jq -r '.logs[0].events[] | select(.type == "wasm").attributes[] | select(.key == "vault_address").value')
  local lp_address=$(echo $res | jq -r '.logs[0].events[] | select(.type == "wasm").attributes[] | select(.key == "lp_address").value')
  local code_ids=($(echo $res | jq -r '.logs[0].events[] | select(.type == "instantiate").attributes[] | select(.key == "code_id").value'))

  # Store on output file
  tmpfile=$(mktemp)
  jq -r --arg asset $asset --arg vault_address $vault_address --arg lp_address $lp_address --arg vault_code_id ${code_ids[0]} --arg lp_code_id ${code_ids[1]} '.vaults += [{asset: $asset, vault_address: $vault_address, lp_address: $lp_address, vault_code_id: $vault_code_id, lp_code_id: $lp_code_id }]' $output_file >$tmpfile
  mv $tmpfile $output_file

  # Add additional deployment information
  date=$(date -u +"%Y-%m-%dT%H:%M:%S%z")
  tmpfile=$(mktemp)
  jq --arg date $date --arg chain_id $CHAIN_ID --arg vault_factory_addr $vault_factory_addr '. + {date: $date , chain_id: $chain_id, vault_factory_addr: $vault_factory_addr}' $output_file >$tmpfile
  mv $tmpfile $output_file

  echo -e "\n**** Created $asset vault on $CHAIN_ID successfully ****\n"
  jq '.' $output_file
}

if [ -z $1 ]; then
  display_usage
  exit 0
fi

# get args
optstring=':c:v:h'
while getopts $optstring arg; do
  case "$arg" in
  c)
    chain=$OPTARG
    source $deployment_script_dir/deploy_env/chain_env.sh
    init_chain_env $OPTARG
    ;;
  v)
    source $deployment_script_dir/wallet_importer.sh
    import_deployer_wallet $chain

    # read vault config from file $OPTARG
    read_vault_config $OPTARG && create_vault
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
