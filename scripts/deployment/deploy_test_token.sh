#!/bin/bash
set -e

# Import the deploy_liquidity_hub script
deployment_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Displays tool usage
function display_usage() {
  echo "WW Test token deployer"
  echo -e "\nUsage:./deploy_test_token.sh [flags].\n"
  echo -e "Available flags:\n"
  echo -e "  -h \thelp"
  echo -e "  -c \tThe chain where you want to deploy (juno|juno-testnet|terra|terra-testnet|... check chain_env.sh for the complete list of supported chains)"
}

function make_symbol() {
  alphabet=({a..z})
  random_index=$(shuf -i 0-25 -n1)
  random_letter=${alphabet[$random_index]}
  symbol="willy"
  symbol+=$random_letter
}

function deploy_token() {
  mkdir -p $project_root_path/scripts/deployment/output
  output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_test_tokens.json
  deployment_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json

  if [[ ! -f "$output_file" ]]; then
    # create file to dump results into
    echo '{"test_tokens": []}' | jq '.' >$output_file
  fi

  make_symbol
  amount=1000000000000000000000
  decimals=6
  token_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_token.wasm") | .code_id' $deployment_file)
  instantiate_msg='{"name":"'$symbol'","symbol":"'$symbol'","decimals":'$decimals',"initial_balances":[{"address":"'$deployer_address'","amount":"'$amount'"}],"mint":{"minter":"'$deployer_address'","cap":"'$amount'"}}'

  echo "Creating test token with the following configuration:"
  echo "Symbol: $symbol"
  echo "Amount: $amount"
  echo "Decimals: $decimals"
  echo -e "Minted to: $deployer_address\n"

  local res=$($BINARY tx wasm instantiate $token_code_id "$instantiate_msg" --label "$symbol token" --no-admin $TXFLAG --from $deployer_address)
  echo $res

  local token_address=($(echo $res | jq -r '.logs[0].events[] | select(.type == "instantiate").attributes[] | select(.key == "_contract_address").value'))
  local tx_hash=($(echo $res | jq -r '.txhash'))

  # Store on output file
  tmpfile=$(mktemp)
  jq -r --arg token_address $token_address --arg code_id $token_code_id --arg symbol $symbol --arg amount $amount --arg decimals $decimals --arg minted_to $deployer_address --arg tx_hash $tx_hash '.test_tokens += [{token_address: $token_address, code_id: $code_id, symbol: $symbol, decimals: $decimals, amount: $amount, minted_to: $minted_to, tx_hash: $tx_hash}]' $output_file >$tmpfile
  mv $tmpfile $output_file

  # Add additional deployment information
  date=$(date -u +"%Y-%m-%dT%H:%M:%S%z")
  tmpfile=$(mktemp)
  jq --arg date $date --arg chain_id $CHAIN_ID --arg minted_to $deployer_address '. + {date: $date , chain_id: $chain_id, minted_to: $minted_to}' $output_file >$tmpfile
  mv $tmpfile $output_file

  echo -e "\n**** Created test token $symbol on $CHAIN_ID successfully ****\n"
  jq '.' $output_file
}

if [ -z $1 ]; then
  display_usage
  exit 0
fi

# get args
optstring=':c:h'
while getopts $optstring arg; do
  case "$arg" in
  c)
    chain=$OPTARG
    source $deployment_script_dir/deploy_env/chain_env.sh
    init_chain_env $chain
    source $deployment_script_dir/wallet_importer.sh
    import_deployer_wallet $chain

    deploy_token
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
