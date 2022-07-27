#!/bin/bash
set -e

projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

source <(cat "$projectRootPath"/scripts/deployment/deploy_env/testnets/juno.env)
DEPLOYER='ww_deployer_wallet'

# import the deployer wallet
export MNEMONIC=$(cat "$projectRootPath"/scripts/deployment/deploy_env/mnemonics/juno_deployer_mnemonic.txt)

# verify if the deployer wallet has already been imported
if ! $BINARY keys show $DEPLOYER > /dev/null 2>&1; then
    # wallet needs to be imported
    echo "Importing $DEPLOYER into $BINARY..."
    echo $MNEMONIC | $BINARY keys add $DEPLOYER --recover > /dev/null 2>&1
fi

CONTRACTS_STORAGE_OUTPUT='{"contracts": []}'
OUTPUT_PATH="$projectRootPath"/scripts/deployment/output/"$CHAIN_ID"_contracts.json

# Store all artifacts on chain
for artifact in "$projectRootPath"/artifacts/*.wasm; do
  echo "Storing $(basename $artifact) on $CHAIN_ID..."
  RES=$($BINARY tx wasm store $artifact $TXFLAG --from $DEPLOYER)
  CODE_ID=$(echo $RES | jq -r '.logs[0].events[-1].attributes[0].value')
  CONTRACTS_STORAGE_OUTPUT=$(jq --arg artifact "$(basename "$artifact")" --arg code_id "$CODE_ID" '.contracts[.contracts|length] |= . + {wasm: $artifact, code_id: $code_id}' $CONTRACTS_STORAGE_OUTPUT)

  # Download the wasm binary from the chain and compare it to the original one
  $BINARY query wasm code $CODE_ID --node $RPC download.wasm
  # The two binaries should be identical
  diff $artifact download.wasm
done

#echo $CONTRACTS_STORAGE_OUTPUT | jq '.' > $OUTPUT_PATH
echo "Stored artifacts on $CHAIN_ID successfully!"

echo "Initializing the Liquidity Hub on $CHAIN_ID..."
deployer_address=$($BINARY keys show $DEPLOYER --output json | jq -r '.address')

echo "Initializing the Fee Collector..."

# Prepare the instantiation message
INIT='{}'
# Instantiate the contract
CODE_ID=$(jq -r '.contracts[] | select (.wasm == "fee_collector.wasm") | .code_id' $OUTPUT_PATH)
$BINARY tx wasm instantiate $CODE_ID "$INIT" --from $DEPLOYER --label "fc" $TXFLAG --admin $deployer_address

# Get contract address
CONTRACT_ADDRESS=$($BINARY query wasm list-contract-by-code $CODE_ID --node $RPC --output json | jq -r '.contracts[-1]')
echo $CONTRACT_ADDRESS

# Append contract_address to output file
TMP=$(mktemp)

jq -r --arg contract_address $CONTRACT_ADDRESS '.contracts[] | select (.wasm == "fee_collector.wasm") |= . + {contract_address: $contract_address}' $OUTPUT_PATH | jq -n '.contracts |= [inputs]' > $TMP
mv $TMP $OUTPUT_PATH && rm $TMP
