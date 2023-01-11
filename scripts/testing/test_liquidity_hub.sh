#!/bin/bash
set -e

testing_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)
deployment_script_dir=$project_root_path/scripts/deployment
tx_delay=8s

# Displays tool usage
function display_usage() {
  echo "WW Liquidity Hub Tester"
  echo -e "\nUsage:./test_liquidity_hub.sh [flags]. Two flags should be used, -c to specify the chain and -t to specify what should be tested."
  echo -e "To test the WW LH the contracts should be stored and initialized first, see deploy_liquidity_hub.sh.\n"
  echo -e "Available flags:\n"
  echo -e "  -h \thelp"
  echo -e "  -c \tThe chain where you want to test (juno|juno-testnet|terra|terra-testnet|... check chain_env.sh for the complete list of supported chains)"
  echo -e "  -t \tWhat to test (all|pool-network|vault-network|fee-collector|pool-factory|pool-router|vault-factory|vault-router)"
}

function create_pool_input_files() {
  test_tokens_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_test_tokens.json

  # create a bash array for token_address in test_token_file with jq
  mapfile token_addresses < <(jq -r '.test_tokens[] | .token_address' $test_tokens_file)

  # create a pool with a native token
  token_address=${token_addresses[0]}
  token_address=$(echo "$token_address" | sed -e 's/\ *$//g') #remove trailing space
  echo '{
                            "protocol_fee": "0.01",
                            "burn_fee": "0.02",
                            "swap_fee": "0.03",
                            "assets": [
                              {
                                "asset": "'$DENOM'",
                                "decimals": "6",
                                "is_native": true
                              },
                              {
                                "asset": "'$token_address'",
                                "decimals": "6",
                                "is_native": false
                              }
                            ]
                          }' | jq '.' >$deployment_script_dir/input/pool-test-0.json

  # create cw20 pools
  for i in $(seq 0 1); do
    token_address_0=${token_addresses[i]}
    token_address_1=${token_addresses[i + 1]}
    token_address_0=$(echo "$token_address_0" | sed -e 's/\ *$//g') #remove trailing space
    token_address_1=$(echo "$token_address_1" | sed -e 's/\ *$//g') #remove trailing space
    echo '{
                                "protocol_fee": "0.01",
                                "burn_fee": "0.02",
                                "swap_fee": "0.03",
                                "assets": [
                                  {
                                    "asset": "'$token_address_0'",
                                    "decimals": "6",
                                    "is_native": false
                                  },
                                  {
                                    "asset": "'$token_address_1'",
                                    "decimals": "6",
                                    "is_native": false
                                  }
                                ]
                              }' | jq '.' >$deployment_script_dir/input/pool-test-$((i + 1)).json
  done
}

#function query_pool_fees() {
#  contracts_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json
#
#  if [ $# -eq 1 ]; then
#    local factory_type=$1
#    if [[ "$factory_type" == "pool" ]]; then
#      local factory=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $contracts_file)
#    elif [[ "$factory_type" == "vault" ]]; then
#      local factory=$(jq -r '.contracts[] | select (.wasm == "vault_factory.wasm") | .contract_address' $contracts_file)
#    else
#      echo "query_pool_fees requires a factory type (pool|vault)"
#      exit 1
#    fi
#  else
#    echo "query_pool_fees requires a factory type (pool|vault)"
#    exit 1
#  fi
#
#  fee_collector=$(jq -r '.contracts[] | select (.wasm == "fee_collector.wasm") | .contract_address' $contracts_file)
#  query='{"fees":{"query_fees_for":{"factory":{"factory_addr":"'$factory'","factory_type":{"'$factory_type'":{}}}},"all_time":false}}'
#  echo $($BINARY query wasm contract-state smart $fee_collector "$query" --node $RPC -o json)
#}
#
#function check_fees() {
#  if [ $# -eq 1 ]; then
#    local json=$(echo $1 | jq -c '.')
#
#    echo -------------------------HERE----------------------
#    echo $json | jq '.'
#    echo -----------------------------THERE------------------
#  else
#    echo "check_fees requires a json type after calling query_pool_fees"
#    exit 1
#  fi
#
#  local total_amount=0
#  for element in $(echo "$json" | jq -c '.data[]'); do
#    local amount=$(echo -r "$element" | jq '.amount')
#    if [[ $amount -gt 0 ]]; then
#      # increment the total_amount
#      total_amount=$((total_amount + amount))
#    fi
#  done
#  echo "total amountis : $total_amount"
#  echo $total_amount
#}

function test_pool_network() {
  contracts_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json
  local fee_collector=$(jq -r '.contracts[] | select (.wasm == "fee_collector.wasm") | .contract_address' $contracts_file)

  # deploy some tokens
  for i in {1..3}; do
    $deployment_script_dir/deploy_test_token.sh -c $chain
    echo "uncomment"
  done
  test_tokens_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_test_tokens.json

  # create some pools
  create_pool_input_files
  echo "uncomment"
  for i in {0..2}; do
    $deployment_script_dir/deploy_pool.sh -c $chain -p $deployment_script_dir/input/pool-test-$i.json
    echo "uncomment"
  done

  # provide liquidity to the pools
  pools_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_pools.json
  mapfile pools < <(jq -r '.pools[] | .pool_address' $pools_file)
  mapfile lp_tokens < <(jq -r '.pools[] | .lp_address' $pools_file)

  for i in "${!pools[@]}"; do
    pool_address=${pools[$i]}
    lp_token_address=${lp_tokens[$i]}
    $deployment_script_dir/provide_liquidity.sh -c $chain -p $pool_address -l $lp_token_address -a 1000000000000,1000000000000
    echo "uncomment"
  done

  # query pool fees, should be zero at this point
  local factory=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $contracts_file)
  query='{"fees":{"query_fees_for":{"factory":{"factory_addr":"'$factory'","factory_type":{"pool":{}}}},"all_time":false}}'
  query_res=$($BINARY query wasm contract-state smart $fee_collector "$query" --node $RPC -o json)
  fees=0
  for amount in $(echo "$query_res" | jq -r '.data[].amount'); do
    fees=$((fees + amount))
  done

  #query_res=$(query_pool_fees pool)
  #fees=$(check_fees $query_res)
  #fees=$((fees))
  if [[ "$fees" -ne 0 ]]; then
    echo "The pools shouldn't have collected any fees at this point. Found $fees"
    exit 1
  fi

  # perform some swaps directly via the pools
  echo -e "\nPerforming swaps on pools...\n"
  for i in "${!pools[@]}"; do
    pool_address=$(echo "${pools[$i]}" | sed -e 's/\ *$//g')
    query='{"pool":{}}'
    readarray -t asset_infos < <($BINARY query wasm contract-state smart $pool_address "$query" --node $RPC --output json | jq -c '.data.assets[].info')

    swap_amounts=(1000 70000000 550000000000)
    for asset_info in "${asset_infos[@]}"; do
      if echo "$asset_info" | jq -e '.native_token' &>/dev/null; then
        denom=$(echo "$asset_info" | jq -r '.native_token.denom')

        for i in $(seq 0 2); do
          swap_msg='{"swap":{"offer_asset":{"amount":"'${swap_amounts[i]}'","info":{"native_token":{"denom":"'$denom'"}}}}}'
          $BINARY tx wasm execute $pool_address "$swap_msg" $TXFLAG --from $deployer_address --amount ${swap_amounts[i]}$denom
          sleep $tx_delay
        done
      else
        cw20_contract=$(echo "$asset_info" | jq -r '.token.contract_addr')
        base64_msg=$(echo '{"swap":{}}' | base64)

        for i in $(seq 0 2); do
          swap_msg='{"send":{"contract":"'$pool_address'","amount":"'${swap_amounts[i]}'","msg":"'$base64_msg'"}}'
          $BINARY tx wasm execute $cw20_contract "$swap_msg" $TXFLAG --from $deployer_address
          sleep $tx_delay
        done
      fi
    done
  done

  # perform some swaps through the router
  echo -e "\nPerforming swaps through router...\n"
  local pool_router=$(jq -r '.contracts[] | select (.wasm == "terraswap_router.wasm") | .contract_address' $contracts_file)

  # swap native -> cw20.0 -> cw20.1
  cw20_0=$(echo "${token_addresses[0]}" | sed -e 's/\ *$//g')
  cw20_1=$(echo "${token_addresses[1]}" | sed -e 's/\ *$//g')
  cw20_2=$(echo "${token_addresses[2]}" | sed -e 's/\ *$//g')

  swap_msg='{"execute_swap_operations":{"operations":[{"terra_swap":{"offer_asset_info":{"native_token":{"denom":"'$DENOM'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_0'"}}}},{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_0'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_1'"}}}}]}}'
  $BINARY tx wasm execute $pool_router "$swap_msg" $TXFLAG --from $deployer_address --amount 10000000000$DENOM
  sleep $tx_delay

  # swap cw20.2 -> cw20.1 -> cw20.0
  ## give allowance to router here first
  query='{"balance":{"address":"'$deployer_address'"}}'
  balance=$($BINARY query wasm contract-state smart $cw20_2 "$query" --node $RPC -o json | jq -r '.data.balance')
  increase_allowance_msg='{"increase_allowance":{"spender":"'$pool_router'","amount":"'$balance'"}}'
  $BINARY tx wasm execute $cw20_2 "$increase_allowance_msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  base64_msg=$(echo '{"execute_swap_operations":{"operations":[{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_2'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_1'"}}}},{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_1'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_0'"}}}}],"minimum_receive":"1000"}}' | base64 -w0)
  swap_msg='{"send":{"contract":"'$pool_router'","amount":"10000000000","msg":"'$base64_msg'"}}'

  $BINARY tx wasm execute $cw20_2 "$swap_msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  # query pool fees, should not be zero at this point
  #query_res=$(query_pool_fees pool)
  #fees=$(check_fees $query_res)
  query='{"fees":{"query_fees_for":{"factory":{"factory_addr":"'$factory'","factory_type":{"pool":{}}}},"all_time":false}}'
  query_res=$($BINARY query wasm contract-state smart $fee_collector "$query" --node $RPC -o json)
  fees=0
  for amount in $(echo "$query_res" | jq -r '.data[].amount'); do
    fees=$((fees + amount))
  done

  if [[ "$fees" -eq 0 ]]; then
    echo "The pools should have collected some fees at this point. Found $fees"
    exit 1
  fi

  # collect those fees
  collect_fees_msg='{"collect_fees":{"collect_fees_for":{"factory":{"factory_addr":"'$factory'","factory_type":{"pool":{}}}}}}'
  $BINARY tx wasm execute $fee_collector "$collect_fees_msg" $TXFLAG --from $deployer_address
  sleep $tx_delay
  # query pool fees, should be zero at this point
  query='{"fees":{"query_fees_for":{"factory":{"factory_addr":"'$factory'","factory_type":{"pool":{}}}},"all_time":false}}'
  query_res=$($BINARY query wasm contract-state smart $fee_collector "$query" --node $RPC -o json)
  fees=0
  for amount in $(echo "$query_res" | jq -r '.data[].amount'); do
    fees=$((fees + amount))
  done

  if [[ "$fees" -ne 0 ]]; then
    echo "The pools should have been emptied by now. Found $fees"
    exit 1
  fi

  # query fees in the fee collector, should have increased
  native_fees=$($BINARY q bank balances $fee_collector --node $RPC -o json | jq -r '.balances[].amount')
  if [[ "$native_fees" -eq 0 ]]; then
    echo "The fee collector should have some native fees collected by now. Found $native_fees"
    exit 1
  fi

  for token_address in ${token_addresses[@]}; do
    token_address=$(echo "$token_address" | sed -e 's/\ *$//g') #remove trailing space

    query='{"balance":{"address":"'$fee_collector'"}}'
    cw20_fees=$($BINARY query wasm contract-state smart $token_address "$query" --node $RPC -o json | jq -r '.data.balance')

    if [[ "$cw20_fees" -eq 0 ]]; then
      echo "The fee collector should have some native fees collected by now. Found $native_fees"
      exit 1
    fi
  done
}

function test_vault_network() {
  echo ""
}

function test_liquidity_hub() {
  echo -e "\nTesting the Liquidity Hub on $CHAIN_ID..."
  test_pool_network
  test_vault_network
}

function cleanup() {
  # remove pool-test-*.json input files
  rm $project_root_path/scripts/deployment/input/pool-test-*.json
}

function test() {
  mkdir -p $project_root_path/scripts/testing/output
  output_file=$project_root_path/scripts/testing/output/"$CHAIN_ID"_liquidity_hub_test_report.json

  if [[ ! -f "$output_file" ]]; then
    # create file to dump results into
    echo '{"tests": []}' | jq '.' >$output_file
    initial_block_height=$(curl -s $RPC/abci_info? | jq -r '.result.response.last_block_height')
  else
    # read from existing deployment file
    initial_block_height=$(jq -r '.initial_block_height' $output_file)
  fi

  case $1 in
  #  pool-network)
  #    init_pool_network
  #    ;;
  #  vault-network)
  #    init_vault_network
  #    ;;
  #  fee-collector)
  #    init_fee_collector
  #    ;;
  #  pool-factory)
  #    init_pool_factory
  #    ;;
  #  pool-router)
  #    init_pool_router
  #    ;;
  #  vault-factory)
  #    init_vault_factory
  #    ;;
  #  vault-router)
  #    init_vault_router
  #    ;;
  *) # test all
    test_liquidity_hub
    ;;
  esac

  output_file=$project_root_path/scripts/testing/output/"$CHAIN_ID"_liquidity_hub_test_report.json

  final_block_height=$(curl -s $RPC/abci_info? | jq -r '.result.response.last_block_height')

  # Add additional test information
  date=$(date -u +"%Y-%m-%dT%H:%M:%S%z")
  tmpfile=$(mktemp)
  jq --arg date $date --arg chain_id $CHAIN_ID --arg deployer_address $deployer_address --arg initial_block_height $initial_block_height --arg final_block_height $final_block_height '. + {date: $date , initial_block_height: $initial_block_height, final_block_height: $final_block_height, chain_id: $chain_id, deployer_address: $deployer_address}' $output_file >$tmpfile
  mv $tmpfile $output_file

  cleanup

  echo -e "\n**** Test successful ****\n"
  jq '.' $output_file
}

if [ -z $1 ]; then
  display_usage
  exit 0
fi

# get args
optstring=':c:t:h'
while getopts $optstring arg; do
  source $deployment_script_dir/wallet_importer.sh

  case "$arg" in
  c)
    chain=$OPTARG
    source $deployment_script_dir/deploy_env/chain_env.sh
    init_chain_env $OPTARG
    if [[ "$chain" = "local" ]]; then
      tx_delay=0.5s
    else
      echo "This kind of testing is supported on local chains only"
      exit 1
      tx_delay=8s
    fi
    ;;
  t)
    import_deployer_wallet $chain
    test $OPTARG
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
