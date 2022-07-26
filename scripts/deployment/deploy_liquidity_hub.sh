#!/bin/bash
set -e

projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

source <(cat "$projectRootPath"/scripts/deployment/deploy_env/testnets/juno.env)
deployer='ww_deployer_wallet'

# import the deployer wallet
export MNEMONIC=$(cat "$projectRootPath"/scripts/deployment/deploy_env/mnemonics/juno_deployer_mnemonic.txt)
echo $MNEMONIC | $BINARY keys add $deployer --recover >/dev/null

JSON_RES='{"contracts": []}'

# Store all artifacts on chain
for artifact in "$projectRootPath"/artifacts/*.wasm; do
  echo "Storing $(basename $artifact) on $CHAIN_ID..."
  RES=$($BINARY tx wasm store $artifact $TXFLAG --from $deployer)
  CODE_ID=$(echo $RES | jq -r '.logs[0].events[-1].attributes[0].value')
  JSON_RES=$(echo "$JSON_RES" | jq --arg artifact "$(basename "$artifact")" --arg code_id "$CODE_ID" '.contracts[.contracts|length] |= . + {wasm: $artifact, code_id: $code_id}')
done

echo $JSON_RES | jq '.' > "$projectRootPath"/scripts/deployment/output/"$CHAIN_ID"_contracts.json
echo "Stored artifacts on $CHAIN_ID successfully!"



