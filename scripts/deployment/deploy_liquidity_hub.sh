#!/bin/bash
set -e

projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

source <(cat "$projectRootPath"/scripts/deployment/deploy_env/testnets/juno.env)
deployer='ww_deployer_wallet'

# import the deployer wallet
export mnemonic=$(cat "$projectRootPath"/scripts/deployment/deploy_env/mnemonics/juno_deployer_mnemonic.txt)

# verify if the deployer wallet has already been imported
if ! $BINARY keys show $deployer >/dev/null 2>&1; then
  # wallet needs to be imported
  echo "Importing $deployer into $BINARY..."
  echo $mnemonic | $BINARY keys add $deployer --recover >/dev/null 2>&1
fi

contracts_storage_output='{"contracts": []}'
output_path="$projectRootPath"/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json

# Store all artifacts on chain
for artifact in "$projectRootPath"/artifacts/*.wasm; do
  echo "Storing $(basename $artifact) on $CHAIN_ID..."
  res=$($BINARY tx wasm store $artifact $TXFLAG --from $deployer)
  code_id=$(echo $res | jq -r '.logs[0].events[-1].attributes[0].value')
  contracts_storage_output=$(echo $contracts_storage_output | jq --arg artifact "$(basename "$artifact")" --arg code_id "$code_id" '.contracts[.contracts|length] |= . + {wasm: $artifact, code_id: $code_id}')

  # Download the wasm binary from the chain and compare it to the original one
  $BINARY query wasm code $code_id --node $RPC downloaded_wasm.wasm
  # The two binaries should be identical
  diff $artifact downloaded_wasm.wasm
  rm downloaded_wasm.wasm
done

echo $contracts_storage_output | jq '.' >$output_path
echo "Stored artifacts on $CHAIN_ID successfully!"

echo "Initializing the Liquidity Hub on $CHAIN_ID..."
deployer_address=$($BINARY keys show $deployer --output json | jq -r '.address')

echo "Initializing the Fee Collector..."

# Prepare the instantiation message
init='{}'
# Instantiate the contract
code_id=$(jq -r '.contracts[] | select (.wasm == "fee_collector.wasm") | .code_id' $output_path)
$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "fc" $TXFLAG --admin $deployer_address

# Get contract address
contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

# Append contract_address to output file
tmpfile=$(mktemp)
jq -r --arg contract_address $contract_address '.contracts[] | select (.wasm == "fee_collector.wasm") |= . + {contract_address: $contract_address}' $output_path | jq -n '.contracts |= [inputs]' >$tmpfile
mv $tmpfile $output_path

echo "Initializing the Pool Factory..."

# Prepare the instantiation message
pair_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_pair.wasm") | .code_id' $output_path)
token_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_token.wasm") | .code_id' $output_path)
fee_collector_addr=$(jq '.contracts[] | select (.wasm == "fee_collector.wasm") | .contract_address' $output_path)

init='{"pair_code_id": '"$pair_code_id"',"token_code_id": '"$token_code_id"', "fee_collector_addr": '"$fee_collector_addr"'}'

# Instantiate the contract
code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .code_id' $output_path)
$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "pf" $TXFLAG --admin $deployer_address

# Get contract address
contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

# Append contract_address to output file
tmpfile=$(mktemp)
jq -r --arg contract_address $contract_address '.contracts[] | select (.wasm == "terraswap_factory.wasm") |= . + {contract_address: $contract_address}' $output_path | jq -n '.contracts |= [inputs]' >$tmpfile
mv $tmpfile $output_path

echo "Initializing the Router..."

# Prepare the instantiation message
terraswap_factory=$(jq '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $output_path)

init='{"terraswap_factory": '"$terraswap_factory"'}'
# Instantiate the contract
code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_router.wasm") | .code_id' $output_path)
$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "r" $TXFLAG --admin $deployer_address

# Get contract address
contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

# Append contract_address to output file
tmpfile=$(mktemp)
jq -r --arg contract_address $contract_address '.contracts[] | select (.wasm == "terraswap_router.wasm") |= . + {contract_address: $contract_address}' $output_path | jq -n '.contracts |= [inputs]' >$tmpfile
mv $tmpfile $output_path

echo "Deployment successful!"
jq '.' $output_path
