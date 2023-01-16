#!/bin/bash
set -e

deployment_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Displays tool usage
function display_usage() {
  echo "WW Liquidity Provider"
  echo -e "\nUsage:./provide_liquidity.sh [flags].\n"
  echo -e "Available flags:\n"
  echo -e "  -h \thelp"
  echo -e "  -c \tThe chain where you want to deploy (juno|juno-testnet|terra|terra-testnet|... check chain_env.sh for the complete list of supported chains)"
  echo -e "  -p \tPool address."
  echo -e "  -a \tAmount of tokens to provide separated by comma, for asset 0 and asset 1."
}

function provide_liquidity() {
  mkdir -p $project_root_path/scripts/deployment/output

  # query assets in the pool
  query='{"pool":{}}'
  readarray -t assets < <($BINARY query wasm contract-state smart $pool_address "$query" --node $RPC --output json | jq -c '.data.assets[]')

  assets[0]=$(echo ${assets[0]} | jq -c ' .amount |= "'${amounts[0]}'"')
  assets[1]=$(echo ${assets[1]} | jq -c ' .amount |= "'${amounts[1]}'"')

  # increase allowance for cw20 tokens looping through assets and checking for info.token.contract_addr
  for asset in "${assets[@]}"; do
    cw20_contract=$(echo $asset | jq -r '.info.token.contract_addr')
    if [[ "$cw20_contract" != "null" ]]; then
      # query balance of deployer address and allow the maximum amount
      query='{"balance":{"address":"'$deployer_address'"}}'
      balance=$($BINARY query wasm contract-state smart $cw20_contract "$query" --node $RPC --output json | jq -r '.data.balance')
      echo -e "Increasing allowance for token $cw20_contract\n"

      increase_allowance_msg='{"increase_allowance":{"spender":"'$pool_address'","amount":"'$balance'"}}'
      $BINARY tx wasm execute $cw20_contract "$increase_allowance_msg" $TXFLAG --from $deployer_address
    fi
  done

  provide_liquidity_msg='{"provide_liquidity":{"assets":['${assets[0]}','${assets[1]}']}}'

  # build amount flag based on the assets to be provided
  amount_denom_pairs=()
  while read -r amount denom; do
    amount_denom_pairs+=("$amount$denom")
  done < <(jq -r '.provide_liquidity.assets[] | select(.info.native_token) | .amount + " " + .info.native_token.denom' <<<"$provide_liquidity_msg")

  if [ ${#amount_denom_pairs[@]} -ne 0 ]; then
    amount_flag="--amount $(
      IFS=,
      echo "${amount_denom_pairs[*]}"
    )"
    IFS=' '
  else
    amount_flag=""
  fi

  echo -e "\nProviding liquidity to:"
  echo "Pool: $pool_address"
  echo "Asset 0: ${assets[0]}"
  echo "Asset 1: ${assets[1]}"

  local res=$($BINARY tx wasm execute $pool_address "$provide_liquidity_msg" $TXFLAG $amount_flag --from $deployer_address)
  echo $res
  sleep $tx_delay

  echo -e "\n**** Provided liquidity successfully ****\n"
}

if [ -z $1 ]; then
  display_usage
  exit 0
fi

# get args
optstring=':c:p:f:a:h'
while getopts $optstring arg; do
  case "$arg" in
  c)
    chain=$OPTARG
    source $deployment_script_dir/deploy_env/chain_env.sh
    init_chain_env $OPTARG
    source $deployment_script_dir/wallet_importer.sh
    import_deployer_wallet $chain

    if [[ "$chain" = "local" ]]; then
      tx_delay=0.5s
    else
      tx_delay=8s
    fi
    ;;
  p)
    pool_address=$OPTARG
    ;;
  a)
    readarray -t amounts < <(awk -F',' '{ for( i=1; i<=NF; i++ ) print $i }' <<<"$OPTARG")
    provide_liquidity
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
