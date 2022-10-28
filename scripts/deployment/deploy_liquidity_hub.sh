#!/bin/bash
set -e

# Displays tool usage
function display_usage() {
  echo "Liquidity Hub Deployer"
  echo -e "\nUsage:\./deploy_liquidity_hub.sh [flags]\n"
  echo -e "Available flags:\n"
  echo -e "  -h \thelp"
  echo -e "  -c \tThe chain where you want to deploy (juno|juno-testnet|terra|terra-testnet)"
}

# Initializes chain env variables
function init_chain_env() {
  case $chain in

  local)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/local.env)
    ;;

  juno)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/mainnets/juno.env)
    ;;

  juno-testnet)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/juno.env)
    ;;

  terra)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/mainnets/terra.env)
    ;;

  terra-testnet)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/terra.env)
    ;;

  archway-testnet)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/archway.env)
    ;;

  chihuahua)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/mainnets/chihuahua.env)
    ;;

  \
    *)
    echo "Network $chain not defined"
    exit 1
    ;;
  esac

  source <(cat "$project_root_path"/scripts/deployment/deploy_env/base.env)
}

function import_deployer_wallet() {
  if # import the deployer wallet
    [[ "$(echo ${chain##*-})" = "testnet" ]]
  then
    deployer='deployer_wallet_testnet'
    export mnemonic=$(cat "$project_root_path"/scripts/deployment/deploy_env/mnemonics/deployer_mnemonic_testnet.txt)
  else
    deployer='deployer_wallet'
    export mnemonic=$(cat "$project_root_path"/scripts/deployment/deploy_env/mnemonics/deployer_mnemonic.txt)
  fi

  # verify if the deployer wallet has already been imported
  if ! $BINARY keys show $deployer >/dev/null 2>&1; then
    # wallet needs to be imported
    echo "Importing $deployer into $BINARY..."
    echo $mnemonic | $BINARY keys add $deployer --recover >/dev/null 2>&1
  fi

  deployer_address=$($BINARY keys show $deployer --output json | jq -r '.address')
}

function store_artifacts_on_chain() {
  for artifact in "$project_root_path"/artifacts/*.wasm; do
    echo "Storing $(basename $artifact) on $CHAIN_ID..."
    # Get contract version for storing purposes
    contract_path=$(find "$project_root_path" -iname $(cut -d . -f 1 <<<$(basename $artifact)) -type d)
    version=$(cat ''"$contract_path"'/Cargo.toml' | awk -F= '/^version/ { print $2 }')
    version="${version//\"/}"

    res=$($BINARY tx wasm store $artifact $TXFLAG --from $deployer)
    code_id=$(echo $res | jq -r '.logs[0].events[-1].attributes[0].value')

    contracts_storage_output=$(echo $contracts_storage_output | jq --arg artifact $(basename "$artifact") --arg code_id $code_id --arg version $version '.contracts[.contracts|length] |= . + {wasm: $artifact, code_id: $code_id, version: $version}')

    # Download the wasm binary from the chain and compare it to the original one
    echo -e "Verifying integrity of wasm artifact on chain...\n"
    $BINARY query wasm code $code_id --node $RPC downloaded_wasm.wasm >/dev/null 2>&1
    # The two binaries should be identical
    diff $artifact downloaded_wasm.wasm
    rm downloaded_wasm.wasm
    sleep $tx_delay
  done

  echo $contracts_storage_output | jq '.' >$output_path
  echo -e "\n**** Stored artifacts on $CHAIN_ID successfully ****\n"
}

function append_contract_address_to_output() {
  if [ $# -eq 2 ]; then
    local contract_address=$1
    local wasm_file_name=$2
  else
    echo "append_contract_to_output needs the contract_address and wasm_file_name"
    exit 1
  fi

  tmpfile=$(mktemp)
  jq -r --arg contract_address $contract_address --arg wasm_file_name $wasm_file_name '.contracts[] | select (.wasm == $wasm_file_name) |= . + {contract_address: $contract_address}' $output_path | jq -n '.contracts |= [inputs]' >$tmpfile
  mv $tmpfile $output_path
}

function init_fee_collector() {
  echo -e "\nInitializing the Fee Collector..."

  # Prepare the instantiation message
  init='{}'
  # Instantiate the contract
  code_id=$(jq -r '.contracts[] | select (.wasm == "fee_collector.wasm") | .code_id' $output_path)
  $BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Fee Collector" $TXFLAG --admin $deployer_address

  # Get contract address
  contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

  # Append contract_address to output file
  append_contract_address_to_output $contract_address 'fee_collector.wasm'
  sleep $tx_delay
}

function init_pool_factory() {
  echo -e "\nInitializing the Pool Factory..."

  # Prepare the instantiation message
  pair_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_pair.wasm") | .code_id' $output_path)
  token_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_token.wasm") | .code_id' $output_path)
  fee_collector_addr=$(jq '.contracts[] | select (.wasm == "fee_collector.wasm") | .contract_address' $output_path)

  init='{"pair_code_id": '"$pair_code_id"',"token_code_id": '"$token_code_id"', "fee_collector_addr": '"$fee_collector_addr"'}'

  # Instantiate the contract
  code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .code_id' $output_path)
  $BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Pool Factory" $TXFLAG --admin $deployer_address

  # Get contract address
  contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

  # Append contract_address to output file
  append_contract_address_to_output $contract_address 'terraswap_factory.wasm'
  sleep $tx_delay
}

function init_pool_router() {
  echo -e "\nInitializing the Pool Router..."

  # Prepare the instantiation message
  terraswap_factory=$(jq '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $output_path)

  init='{"terraswap_factory": '"$terraswap_factory"'}'
  # Instantiate the contract
  code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_router.wasm") | .code_id' $output_path)
  $BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Pool Router" $TXFLAG --admin $deployer_address

  # Get contract address
  contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

  # Append contract_address to output file
  append_contract_address_to_output $contract_address 'terraswap_router.wasm'
  sleep $tx_delay
}

function init_vault_factory() {
  echo -e "\nInitializing the Vault Factory..."

  # Prepare the instantiation message
  vault_id=$(jq -r '.contracts[] | select (.wasm == "vault.wasm") | .code_id' $output_path)

  init='{"owner": "'$deployer_address'", "vault_id": '"$vault_id"', "token_id": '"$token_code_id"', "fee_collector_addr": '"$fee_collector_addr"'}'

  # Instantiate the contract
  code_id=$(jq -r '.contracts[] | select (.wasm == "vault_factory.wasm") | .code_id' $output_path)
  $BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Vault Factory" $TXFLAG --admin $deployer_address

  # Get contract address
  contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

  # Append contract_address to output file
  append_contract_address_to_output $contract_address 'vault_factory.wasm'
  sleep $tx_delay
}

function init_vault_router() {
  echo -e "\nInitializing the Vault Router..."

  # Prepare the instantiation message
  vault_factory_addr=$(jq '.contracts[] | select (.wasm == "vault_factory.wasm") | .contract_address' $output_path)

  init='{"owner": "'$deployer_address'", "vault_factory_addr": '"$vault_factory_addr"'}'

  # Instantiate the contract
  code_id=$(jq -r '.contracts[] | select (.wasm == "vault_router.wasm") | .code_id' $output_path)
  $BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Vault Router" $TXFLAG --admin $deployer_address

  # Get contract address
  contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

  # Append contract_address to output file
  append_contract_address_to_output $contract_address 'vault_router.wasm'
  sleep $tx_delay
}

function init_pool_network() {
  init_fee_collector
  init_pool_factory
  init_pool_router
}

function init_vault_network() {
  init_vault_factory
  init_vault_router
}

function init_liquidity_hub() {
  echo -e "\nInitializing the Liquidity Hub on $CHAIN_ID..."
  init_pool_network
  init_vault_network
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

project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)
tx_delay=8s

init_chain_env
import_deployer_wallet

# create file to dump results into
contracts_storage_output='{"contracts": []}'
mkdir -p "$project_root_path"/scripts/deployment/output
output_path="$project_root_path"/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json

initial_block_height=$(curl -s $RPC/abci_info? | jq -r '.result.response.last_block_height')

store_artifacts_on_chain
init_liquidity_hub

final_block_height=$(curl -s $RPC/abci_info? | jq -r '.result.response.last_block_height')

# Add additional deployment information
date=$(date -u +"%Y-%m-%dT%H:%M:%S%z")
tmpfile=$(mktemp)
jq --arg date $date --arg chain_id $CHAIN_ID --arg deployer_address $deployer_address --arg initial_block_height $initial_block_height --arg final_block_height $final_block_height '. + {date: $date , initial_block_height: $initial_block_height, final_block_height: $final_block_height, chain_id: $chain_id, deployer_address: $deployer_address}' $output_path >$tmpfile
mv $tmpfile $output_path

echo -e "\n**** Deployment successful ****\n"
jq '.' $output_path
