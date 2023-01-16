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
  echo -e "There has to be an instance of a local blockchain running, see launch_local_chain.sh.\n"
  echo -e "Available flags:\n"
  echo -e "  -h \thelp"
  echo -e "  -c \tThe chain where you want to test (juno|juno-testnet|terra|terra-testnet|... check chain_env.sh for the complete list of supported chains)"
  echo -e "  -t \tWhat to test (all|migrations)"
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

function create_vault_input_files() {
  test_tokens_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_test_tokens.json

  # create a bash array for token_address in test_token_file with jq
  mapfile token_addresses < <(jq -r '.test_tokens[] | .token_address' $test_tokens_file)

  # create a vault with a native token
  token_address=${token_addresses[0]}
  token_address=$(echo "$token_address" | sed -e 's/\ *$//g') #remove trailing space

  echo '{
                            "protocol_fee": "0.01",
                            "burn_fee": "0.02",
                            "flash_loan_fee": "0.03",
                            "is_native": true,
                            "asset": "'$DENOM'"
                          }' | jq '.' >$deployment_script_dir/input/vault-test-0.json

  echo '{
                            "protocol_fee": "0.01",
                            "burn_fee": "0.02",
                            "flash_loan_fee": "0.03",
                            "is_native": false,
                            "asset": "'$token_address'"
                          }' | jq '.' >$deployment_script_dir/input/vault-test-1.json

}

function smoke_test_pool_network() {
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

  for i in "${!pools[@]}"; do
    pool_address=${pools[$i]}
    $deployment_script_dir/provide_liquidity.sh -c $chain -p $pool_address -a 1000000000000,1000000000000
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
      echo "The fee collector should have some native fees collected by now. Found $cw20_fees"
      exit 1
    fi
  done

  # aggregate fees

  echo -e "\nAdding pool router address to fee collector...\n"
  # first, add router address to fee collector and swap routes to router
  update_config_msg='{"update_config": {"pool_router": "'$pool_router'"}}'
  $BINARY tx wasm execute $fee_collector "$update_config_msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  # create swap routes
  echo -e "\nAdd swap routes to pool router...\n"
  add_swap_routes_msg='{"add_swap_routes":{"swap_routes":[{"offer_asset_info":{"token":{"contract_addr":"'$cw20_0'"}},"ask_asset_info":{"native_token":{"denom":"'$DENOM'"}},"swap_operations":[{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_0'"}},"ask_asset_info":{"native_token":{"denom":"'$DENOM'"}}}}]},{"offer_asset_info":{"token":{"contract_addr":"'$cw20_1'"}},"ask_asset_info":{"native_token":{"denom":"'$DENOM'"}},"swap_operations":[{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_1'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_0'"}}}},{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_0'"}},"ask_asset_info":{"native_token":{"denom":"'$DENOM'"}}}}]},{"offer_asset_info":{"token":{"contract_addr":"'$cw20_2'"}},"ask_asset_info":{"native_token":{"denom":"'$DENOM'"}},"swap_operations":[{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_2'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_1'"}}}},{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_1'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_0'"}}}},{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_0'"}},"ask_asset_info":{"native_token":{"denom":"'$DENOM'"}}}}]},{"offer_asset_info":{"native_token":{"denom":"'$DENOM'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_1'"}},"swap_operations":[{"terra_swap":{"offer_asset_info":{"native_token":{"denom":"'$DENOM'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_0'"}}}},{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_0'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_1'"}}}}]},{"offer_asset_info":{"token":{"contract_addr":"'$cw20_0'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_1'"}},"swap_operations":[{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_0'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_1'"}}}}]},{"offer_asset_info":{"token":{"contract_addr":"'$cw20_2'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_1'"}},"swap_operations":[{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_2'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_1'"}}}}]}]}}'
  $BINARY tx wasm execute $pool_router "$add_swap_routes_msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  # aggregate everything to the native token
  echo -e "\nAggregate tokens...\n"
  aggregate_fees_msg='{"aggregate_fees":{"asset_info": {"native_token": {"denom": "'$DENOM'"}},"aggregate_fees_for":{"factory":{"factory_addr":"'$factory'","factory_type":{"pool":{}}}}}}'
  $BINARY tx wasm execute $fee_collector "$aggregate_fees_msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  # check that native fees are more than before and that the other tokens are zero
  aggregated_fees=$($BINARY q bank balances $fee_collector --node $RPC -o json | jq -r '.balances[].amount')
  if [[ "$aggregated_fees" -lt $native_fees ]]; then
    echo "The native token fees should be greater than before it was aggregated. Found $aggregated_fees, before it was $native_fees."
    exit 1
  fi

  for token_address in ${token_addresses[@]}; do
    token_address=$(echo "$token_address" | sed -e 's/\ *$//g') #remove trailing space

    query='{"balance":{"address":"'$fee_collector'"}}'
    cw20_fees=$($BINARY query wasm contract-state smart $token_address "$query" --node $RPC -o json | jq -r '.data.balance')

    if [[ "$cw20_fees" -ne 0 ]]; then
      echo "The balance for the token $token_address should be zero by now. Found $cw20_fees"
      exit 1
    fi
  done

  # aggregate everything to the cw20_1 token
  echo -e "\nAggregate tokens...\n"
  aggregate_fees_msg='{"aggregate_fees":{"asset_info": {"token": {"contract_addr": "'$cw20_1'"}},"aggregate_fees_for":{"factory":{"factory_addr":"'$factory'","factory_type":{"pool":{}}}}}}'
  $BINARY tx wasm execute $fee_collector "$aggregate_fees_msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  # check that native fees, should be zero now
  aggregated_fees=$($BINARY q bank balances $fee_collector --node $RPC -o json | jq -r '.balances[].amount')
  if [[ "$aggregated_fees" -ne 0 ]]; then
    echo "The native token fees should be zero now. Found $aggregated_fees."
    exit 1
  fi

  for token_address in ${token_addresses[@]}; do
    token_address=$(echo "$token_address" | sed -e 's/\ *$//g') #remove trailing space

    query='{"balance":{"address":"'$fee_collector'"}}'
    cw20_fees=$($BINARY query wasm contract-state smart $token_address "$query" --node $RPC -o json | jq -r '.data.balance')

    # if token_address is cw20_1, then it should have some fees, otherwise zero
    if [[ "$token_address" == "$cw20_1" ]]; then
      if [[ "$cw20_fees" -eq 0 ]]; then
        echo "The balance for the token $token_address should be greater than zero by now. Found $cw20_fees"
        exit 1
      fi
    else
      if [[ "$cw20_fees" -ne 0 ]]; then
        echo "The balance for the token $token_address should be zero by now. Found $cw20_fees"
        exit 1
      fi
    fi
  done
}

function smoke_test_vault_network() {
  local contracts_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json
  local fee_collector=$(jq -r '.contracts[] | select (.wasm == "fee_collector.wasm") | .contract_address' $contracts_file)

  # create some vaults
  create_vault_input_files
  for i in {0..1}; do
    $deployment_script_dir/deploy_vault.sh -c $chain -v $deployment_script_dir/input/vault-test-$i.json
    echo "uncomment"
  done

  # provide liquidity to the vaults
  vaults_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_vaults.json
  mapfile vaults < <(jq -r '.vaults[] | .vault_address' $vaults_file)

  echo -e "\nDeposit into the vaults...\n"

  for i in "${!vaults[@]}"; do
    vault_address=$(echo ${vaults[$i]} | sed -e 's/\ *$//g')
    deposit_msg='{"deposit":{"amount":"1000000000000"}}'

    amount_flag=""
    if [[ $i -eq 0 ]]; then
      amount_flag="--amount 1000000000000$DENOM"
    else
      cw20_0=$(jq -r '.vaults[-1].asset' $vaults_file | sed -e 's/\ *$//g')
      # increase allowance for cw20_0 in second vault

      increase_allowance_msg='{"increase_allowance":{"spender":"'$vault_address'","amount":"1000000000000"}}'
      $BINARY tx wasm execute $cw20_0 "$increase_allowance_msg" $TXFLAG --from $deployer_address
      sleep $tx_delay
    fi

    $BINARY tx wasm execute $vault_address "$deposit_msg" $TXFLAG --from $deployer_address $amount_flag
    sleep $tx_delay
  done

  token_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_token.wasm") | .code_id' $contracts_file)
  pair_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_pair.wasm") | .code_id' $contracts_file)

  # Perform a flash loan

  echo -e "\nCreate pool off balance...\n"
  instantiate_pool_msg='{"asset_infos":[{"native_token":{"denom":"'$DENOM'"}},{"token":{"contract_addr":"'$cw20_0'"}}],"token_code_id": '$token_code_id',"asset_decimals":[6,6],"pool_fees":{"protocol_fee":{"share":"0.02"},"swap_fee":{"share":"0.03"},"burn_fee":{"share":"0.01"}},"fee_collector_addr":"'$fee_collector'"}'
  res=$($BINARY tx wasm instantiate $pair_code_id "$instantiate_pool_msg" --from $deployer_address --label "Off-balance pool" $TXFLAG --admin $deployer_address)
  sleep $tx_delay

  off_balance_pool=$(echo $res | jq -r '.logs[0].events[] | select(.type == "wasm") | .attributes[] | select(.key == "_contract_address") | .value')
  # add off balance liquidity to the pool
  $deployment_script_dir/provide_liquidity.sh -c $chain -p $off_balance_pool -a 1000000000000,300000000000000

  pools_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_pools.json
  balanced_pool=$(jq -r '.pools[0].pool_address' $pools_file | sed -e 's/\ *$//g')

  # balanced pool should have a ratio of about 1.3 while the off balance pool 300.
  # according to the simulation queries, 1k stake gives you about 281718281718 in the off balance pool while 1263067726 in the balanced pool.

  # swap stake in off_balance_pool and it back in balanced_pool

  echo -e "\nPerform flashloan...\n"
  # prepare messages for flashloan
  original_native_token_balance=$($BINARY q bank balances $deployer_address --node $RPC -o json | jq -r '.balances[].amount')
  echo -e "\noriginal_native_token_balance: $original_native_token_balance"
  flash_loan_amount=1000000000

  swap_native_cw20_0_msg=$(echo '{"swap":{"offer_asset":{"amount":"'$flash_loan_amount'","info":{"native_token":{"denom":"'$DENOM'"}}}}}' | base64 -w0)
  expected_cw20_0_amount=281718281718
  swap_cw20_0_native_msg='{"swap":{"offer_asset":{"amount":"'$flash_loan_amount'","info":{"token":{"contract_addr":"'$cw20_0'"}}}}}'
  swap_msg=$(echo '{"swap":{}}' | base64 -w0)
  swap_cw20_0_native_msg=$(echo '{"send":{"amount":"'$expected_cw20_0_amount'","contract":"'$balanced_pool'","msg":"'$swap_msg'"}}' | base64 -w0)

  flash_loan_msg_1='{"wasm":{"execute":{"msg":"'$swap_native_cw20_0_msg'","funds":[{"denom":"'$DENOM'","amount":"'$flash_loan_amount'"}],"contract_addr":"'$off_balance_pool'"}}}'
  flash_loan_msg_2='{"wasm":{"execute":{"msg":"'$swap_cw20_0_native_msg'","funds":[],"contract_addr":"'$cw20_0'"}}}'

  local vault_router=$(jq -r '.contracts[] | select (.wasm == "vault_router.wasm") | .contract_address' $contracts_file)
  flash_loan='{"flash_loan":{"msgs":['$flash_loan_msg_1','$flash_loan_msg_2'],"assets":[{"info":{"native_token":{"denom":"'$DENOM'"}},"amount":"'$flash_loan_amount'"}]}}'

  echo "$BINARY tx wasm execute $vault_router "$flash_loan" $TXFLAG --from $deployer_address"
  $BINARY tx wasm execute $vault_router "$flash_loan" $TXFLAG --from $deployer_address

  native_token_balance_after_flashloan=$($BINARY q bank balances $deployer_address --node $RPC -o json | jq -r '.balances[].amount')

  # compare that the native token balance after flashloan is bigger than the original one
  if [[ "$native_token_balance_after_flashloan" -lt "$original_native_token_balance" ]]; then
    echo "Something went off, the native token balance after flashloan should be higher. Originally was: $original_native_token_balance, now is: $native_token_balance_after_flashloan"
  fi
}

function test_pool_network_queries() {
  # load all necessary variables
  local contracts_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json
  local fee_collector=$(jq -r '.contracts[] | select (.wasm == "fee_collector.wasm") | .contract_address' $contracts_file)
  local pool_factory=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $contracts_file)
  local pool_router=$(jq -r '.contracts[] | select (.wasm == "terraswap_router.wasm") | .contract_address' $contracts_file)
  local vault_factory=$(jq -r '.contracts[] | select (.wasm == "vault_factory.wasm") | .contract_address' $contracts_file)
  pools_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_pools.json
  mapfile pools < <(jq -r '.pools[] | .pool_address' $pools_file)
  vaults_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_vaults.json
  mapfile vaults < <(jq -r '.vaults[] | .vault_address' $vaults_file)
  test_tokens_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_test_tokens.json
  mapfile token_addresses < <(jq -r '.test_tokens[] | .token_address' $test_tokens_file)
  cw20_0=$(echo "${token_addresses[0]}" | sed -e 's/\ *$//g')

  echo -e "\nFee collector queries...\n"

  # fees for pool factory
  query='{"fees":{"query_fees_for":{"factory":{"factory_addr":"'$pool_factory'","factory_type":{"pool":{}}}},"all_time":false}}'
  $BINARY query wasm contract-state smart $fee_collector "$query" --node $RPC -o json | jq -c '.data'
  query='{"fees":{"query_fees_for":{"factory":{"factory_addr":"'$pool_factory'","factory_type":{"pool":{}}}},"all_time":true}}'
  $BINARY query wasm contract-state smart $fee_collector "$query" --node $RPC -o json | jq -c '.data'

  # fees for vault factory
  query='{"fees":{"query_fees_for":{"factory":{"factory_addr":"'$vault_factory'","factory_type":{"vault":{}}}},"all_time":false}}'
  $BINARY query wasm contract-state smart $fee_collector "$query" --node $RPC -o json | jq -c '.data'
  query='{"fees":{"query_fees_for":{"factory":{"factory_addr":"'$vault_factory'","factory_type":{"vault":{}}}},"all_time":true}}'
  $BINARY query wasm contract-state smart $fee_collector "$query" --node $RPC -o json | jq -c '.data'

  # fees for contracts
  query='{"fees":{"query_fees_for":{"contracts":{"contracts":[{"address":"'${pools[0]}'","contract_type":{"pool":{}}},{"address":"'${vaults[0]}'","contract_type":{"vault":{}}}]}},"all_time":false}}'
  $BINARY query wasm contract-state smart $fee_collector "$query" --node $RPC -o json | jq -c '.data'
  query='{"fees":{"query_fees_for":{"contracts":{"contracts":[{"address":"'${pools[0]}'","contract_type":{"pool":{}}},{"address":"'${vaults[0]}'","contract_type":{"vault":{}}}]}},"all_time":true}}'
  $BINARY query wasm contract-state smart $fee_collector "$query" --node $RPC -o json | jq -c '.data'

  # config
  query='{"config":{}}'
  $BINARY query wasm contract-state smart $fee_collector "$query" --node $RPC -o json | jq -c '.data'

  echo -e "\nPool factory queries...\n"

  # config
  query='{"config":{}}'
  $BINARY query wasm contract-state smart $pool_factory "$query" --node $RPC -o json | jq -c '.data'

  # pair
  query='{"pair":{"asset_infos":[{"native_token":{"denom":"'$DENOM'"}},{"token":{"contract_addr":"'$cw20_0'"}}]}}'
  $BINARY query wasm contract-state smart $pool_factory "$query" --node $RPC -o json | jq -c '.data'

  # pairs
  query='{"pairs":{}}'
  $BINARY query wasm contract-state smart $pool_factory "$query" --node $RPC -o json | jq -c '.data'

  echo -e "\nPool queries...\n"

  # config
  query='{"config":{}}'
  $BINARY query wasm contract-state smart ${pools[0]} "$query" --node $RPC -o json | jq -c '.data'

  # pair
  query='{"pair":{}}'
  $BINARY query wasm contract-state smart ${pools[0]} "$query" --node $RPC -o json | jq -c '.data'

  # pool
  query='{"pool":{}}'
  $BINARY query wasm contract-state smart ${pools[0]} "$query" --node $RPC -o json | jq -c '.data'

  # protocol fees
  query='{"protocol_fees":{"asset_id":"'$DENOM'","all_time":false}}'
  $BINARY query wasm contract-state smart ${pools[0]} "$query" --node $RPC -o json | jq -c '.data'
  query='{"protocol_fees":{"asset_id":"'$DENOM'","all_time":true}}'
  $BINARY query wasm contract-state smart ${pools[0]} "$query" --node $RPC -o json | jq -c '.data'

  # burned fees
  query='{"burned_fees":{"asset_id":"'$DENOM'"}}'
  $BINARY query wasm contract-state smart ${pools[0]} "$query" --node $RPC -o json | jq -c '.data'

  # simulation
  query='{"simulation":{"offer_asset":{"info":{"native_token":{"denom":"'$DENOM'"}},"amount":"1000"}}}'
  $BINARY query wasm contract-state smart ${pools[0]} "$query" --node $RPC -o json | jq -c '.data'
  query='{"simulation":{"offer_asset":{"info":{"native_token":{"denom":"'$DENOM'"}},"amount":"500000000000"}}}'
  $BINARY query wasm contract-state smart ${pools[0]} "$query" --node $RPC -o json | jq -c '.data'

  # reverse simulation
  query='{"reverse_simulation":{"ask_asset":{"info":{"native_token":{"denom":"'$DENOM'"}},"amount":"1000"}}}'
  $BINARY query wasm contract-state smart ${pools[0]} "$query" --node $RPC -o json | jq -c '.data'
  query='{"reverse_simulation":{"ask_asset":{"info":{"native_token":{"denom":"'$DENOM'"}},"amount":"500000000000"}}}'
  $BINARY query wasm contract-state smart ${pools[0]} "$query" --node $RPC -o json | jq -c '.data'

  echo -e "\nPool Router queries...\n"

  # config
  query='{"config":{}}'
  $BINARY query wasm contract-state smart $pool_router "$query" --node $RPC -o json | jq -c '.data'

  # simulate swap operations
  query='{"simulate_swap_operations":{"operations":[{"terra_swap":{"offer_asset_info":{"native_token":{"denom":"'$DENOM'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_0'"}}}},{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"'$cw20_0'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_1'"}}}}],"offer_amount":"1000000000"}}'
  $BINARY query wasm contract-state smart $pool_router "$query" --node $RPC -o json | jq -c '.data'
  # simulate reverse swap operations
  query='{"reverse_simulate_swap_operations":{"operations":[{"terra_swap":{"offer_asset_info":{"native_token":{"denom":"'$DENOM'"}},"ask_asset_info":{"token":{"contract_addr":"'$cw20_0'"}}}}],"ask_amount":"1000000000"}}'
  $BINARY query wasm contract-state smart $pool_router "$query" --node $RPC -o json | jq -c '.data'

  #echo -e "\nToken queries...\n"
}

function test_vault_network_queries() {
  # load all necessary variables
  local contracts_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json
  local vault_factory=$(jq -r '.contracts[] | select (.wasm == "vault_factory.wasm") | .contract_address' $contracts_file)
  local vault_router=$(jq -r '.contracts[] | select (.wasm == "vault_router.wasm") | .contract_address' $contracts_file)
  vaults_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_vaults.json
  mapfile vaults < <(jq -r '.vaults[] | .vault_address' $vaults_file)
  test_tokens_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_test_tokens.json
  mapfile token_addresses < <(jq -r '.test_tokens[] | .token_address' $test_tokens_file)
  cw20_0=$(echo "${token_addresses[0]}" | sed -e 's/\ *$//g')

  echo -e "\nVault factory queries...\n"

  # config
  query='{"config":{}}'
  $BINARY query wasm contract-state smart $vault_factory "$query" --node $RPC -o json | jq -c '.data'

  # vaults
  query='{"vaults":{}}'
  $BINARY query wasm contract-state smart $vault_factory "$query" --node $RPC -o json | jq -c '.data'

  # vault
  query='{"vault":{"asset_info":{"native_token":{"denom":"'$DENOM'"}}}}'
  $BINARY query wasm contract-state smart $vault_factory "$query" --node $RPC -o json | jq -c '.data'
  query='{"vault":{"asset_info":{"token":{"contract_addr":"'$cw20_0'"}}}}'
  $BINARY query wasm contract-state smart $vault_factory "$query" --node $RPC -o json | jq -c '.data'

  echo -e "\nVault router queries...\n"

  # config
  query='{"config":{}}'
  $BINARY query wasm contract-state smart $vault_router "$query" --node $RPC -o json | jq -c '.data'

  echo -e "\nVault queries...\n"

  # config
  query='{"config":{}}'
  $BINARY query wasm contract-state smart ${vaults[0]} "$query" --node $RPC -o json | jq -c '.data'
  $BINARY query wasm contract-state smart ${vaults[1]} "$query" --node $RPC -o json | jq -c '.data'

  # get_payback_amount
  query='{"get_payback_amount":{"amount":"10000"}}'
  $BINARY query wasm contract-state smart ${vaults[0]} "$query" --node $RPC -o json | jq -c '.data'
  $BINARY query wasm contract-state smart ${vaults[1]} "$query" --node $RPC -o json | jq -c '.data'

  # protocol_fees
  query='{"protocol_fees":{"all_time":false}}'
  $BINARY query wasm contract-state smart ${vaults[0]} "$query" --node $RPC -o json | jq -c '.data'
  $BINARY query wasm contract-state smart ${vaults[1]} "$query" --node $RPC -o json | jq -c '.data'
  query='{"protocol_fees":{"all_time":true}}'
  $BINARY query wasm contract-state smart ${vaults[0]} "$query" --node $RPC -o json | jq -c '.data'
  $BINARY query wasm contract-state smart ${vaults[1]} "$query" --node $RPC -o json | jq -c '.data'

  # burned_fees
  query='{"burned_fees":{}}'
  $BINARY query wasm contract-state smart ${vaults[0]} "$query" --node $RPC -o json | jq -c '.data'
  $BINARY query wasm contract-state smart ${vaults[1]} "$query" --node $RPC -o json | jq -c '.data'

  # share
  query='{"share":{"amount":"10000"}}'
  $BINARY query wasm contract-state smart ${vaults[0]} "$query" --node $RPC -o json | jq -c '.data'
  $BINARY query wasm contract-state smart ${vaults[1]} "$query" --node $RPC -o json | jq -c '.data'
}

function pool_factory_execute_msgs() {
  local contracts_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json
  local pool_factory=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $contracts_file)
  pools_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_pools.json
  mapfile pools < <(jq -r '.pools[] | .pool_address' $pools_file)
  test_tokens_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_test_tokens.json
  mapfile token_addresses < <(jq -r '.test_tokens[] | .token_address' $test_tokens_file)
  cw20_0=$(echo "${token_addresses[0]}" | sed -e 's/\ *$//g')
  pool=$(echo "${pools[0]}" | sed -e 's/\ *$//g')

  echo -e "\nExecuting missing ExecuteMsgs on pool factory...\n"

  # add_native_decimals
  msg='{"add_native_token_decimals":{"denom":"'$DENOM'","decimals":6}}'
  $BINARY tx wasm execute $pool_factory "$msg" $TXFLAG --from $deployer_address --amount 1$DENOM
  sleep $tx_delay

  # update config
  msg='{"update_config":{"owner":"'$deployer_address'","fee_collector_addr":"'$deployer_address'","token_code_id":123,"pair_code_id":456}}'
  $BINARY tx wasm execute $pool_factory "$msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  # update pair config
  msg='{"update_pair_config":{"pair_addr":"'$pool'","fee_collector_addr":"'$deployer_address'","pool_fees":{"protocol_fee":{"share":"0.005"},"swap_fee":{"share":"0.005"},"burn_fee":{"share":"0.005"}},"feature_toggle":{"withdrawals_enabled":true,"deposits_enabled":false,"swaps_enabled":false}}}'
  $BINARY tx wasm execute $pool_factory "$msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  # remove pair
  msg='{"remove_pair":{"asset_infos":[{"native_token":{"denom":"'$DENOM'"}},{"token":{"contract_addr":"'$cw20_0'"}}]}}'
  $BINARY tx wasm execute $pool_factory "$msg" $TXFLAG --from $deployer_address
  sleep $tx_delay
}

function pool_execute_msgs() {
  pools_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_pools.json
  pool=$(jq -r '.pools[0].pool_address' $pools_file)
  lp=$(jq -r '.pools[0].lp_address' $pools_file)

  echo -e "\nExecuting missing ExecuteMsgs on pool...\n"

  # withdraw
  query='{"balance":{"address":"'$deployer_address'"}}'
  balance=$($BINARY query wasm contract-state smart $lp "$query" --node $RPC -o json | jq -r '.data.balance')

  withdraw=$(echo '{"withdraw_liquidity":{}}' | base64 -w0)
  msg='{"send":{"contract":"'$pool'","amount":"'$balance'","msg":"'$withdraw'"}}'
  $BINARY tx wasm execute $lp "$msg" $TXFLAG --from $deployer_address
  sleep $tx_delay
}

function vault_factory_execute_msgs() {
  # load all necessary variables
  local contracts_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json
  local vault_factory=$(jq -r '.contracts[] | select (.wasm == "vault_factory.wasm") | .contract_address' $contracts_file)
  vaults_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_vaults.json
  mapfile vaults < <(jq -r '.vaults[] | .vault_address' $vaults_file)
  test_tokens_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_test_tokens.json
  mapfile token_addresses < <(jq -r '.test_tokens[] | .token_address' $test_tokens_file)
  cw20_0=$(echo "${token_addresses[0]}" | sed -e 's/\ *$//g')
  vault=$(echo "${vaults[1]}" | sed -e 's/\ *$//g')

  echo -e "\nExecuting missing ExecuteMsgs on vault factory...\n"

  # update_vault_config
  msg='{"update_vault_config":{"vault_addr":"'$vault'","params":{"flash_loan_enabled":false,"deposit_enabled":false,"withdraw_enabled":false,"new_owner":"'$cw20_0'","new_vault_fees":{"protocol_fee":{"share":"0.001"},"flash_loan_fee":{"share":"0.003"},"burn_fee":{"share":"0.001"}},"new_fee_collector_addr":"'$deployer_address'"}}}'
  $BINARY tx wasm execute $vault_factory "$msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  # update_config
  msg='{"update_config":{"fee_collector_addr":"'$deployer_address'","vault_id":123,"token_id":456}}'
  $BINARY tx wasm execute $vault_factory "$msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  # remove vault
  msg='{"remove_vault":{"asset_info":{"native_token":{"denom":"'$DENOM'"}}}}'
  $BINARY tx wasm execute $vault_factory "$msg" $TXFLAG --from $deployer_address
  sleep $tx_delay
  msg='{"remove_vault":{"asset_info":{"token":{"contract_addr":"'$cw20_0'"}}}}'
  $BINARY tx wasm execute $vault_factory "$msg" $TXFLAG --from $deployer_address
  sleep $tx_delay
}

function vault_router_execute_msgs() {
  # load all necessary variables
  local contracts_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json
  local vault_router=$(jq -r '.contracts[] | select (.wasm == "vault_router.wasm") | .contract_address' $contracts_file)
  local random_addr=wasm124gxqh0ek78rde34kl5ng73frnp0tzv9c6v0nq

  echo -e "\nExecuting missing ExecuteMsgs on vault router...\n"

  # update_config
  msg='{"update_config":{"owner":"'$random_addr'","vault_factory_addr":"'$random_addr'"}}'
  $BINARY tx wasm execute $vault_router "$msg" $TXFLAG --from $deployer_address
  sleep $tx_delay
}

function vault_execute_msgs() {
  # load all necessary variables
  vaults_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_vaults.json
  vault=$(jq -r '.vaults[0].vault_address' $vaults_file)
  lp=$(jq -r '.vaults[0].lp_address' $vaults_file)

  echo -e "\nExecuting missing ExecuteMsgs on vault...\n"

  # withdraw
  query='{"balance":{"address":"'$deployer_address'"}}'
  balance=$($BINARY query wasm contract-state smart $lp "$query" --node $RPC -o json | jq -r '.data.balance')

  withdraw=$(echo '{"withdraw":{}}' | base64 -w0)
  msg='{"send":{"contract":"'$vault'","amount":"'$balance'","msg":"'$withdraw'"}}'
  echo "$BINARY tx wasm execute $lp "$msg" $TXFLAG --from $deployer_address"
  $BINARY tx wasm execute $lp "$msg" $TXFLAG --from $deployer_address
  sleep $tx_delay
}

function test_liquidity_hub() {
  echo -e "\nTesting the Liquidity Hub on $CHAIN_ID..."
  smoke_test_pool_network
  smoke_test_vault_network
  test_pool_network_queries
  test_vault_network_queries

  # run execute messages missing in the smoke tests
  pool_factory_execute_msgs
  pool_execute_msgs
  vault_factory_execute_msgs
  vault_router_execute_msgs
  vault_execute_msgs
}

function test_migrations() {
  echo -e "\nMigrating contracts on the Liquidity Hub on $CHAIN_ID..."

  contracts_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json
  local fee_collector=$(jq -r '.contracts[] | select (.wasm == "fee_collector.wasm") | .contract_address' $contracts_file)
  local pool_factory=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $contracts_file)
  local pool_router=$(jq -r '.contracts[] | select (.wasm == "terraswap_router.wasm") | .contract_address' $contracts_file)
  local vault_factory=$(jq -r '.contracts[] | select (.wasm == "vault_factory.wasm") | .contract_address' $contracts_file)
  local vault_router=$(jq -r '.contracts[] | select (.wasm == "vault_router.wasm") | .contract_address' $contracts_file)

  # store all new contracts
  migration_artifacts_path=$project_root_path/scripts/deployment/input/migrations/artifacts
  $project_root_path/scripts/deployment/deploy_liquidity_hub.sh -c $chain -a $migration_artifacts_path -s all

  # migrate contracts

  echo -e "\nMigrating fee collector..."

  code_ids=($(jq -r '.contracts[] | select(.wasm == "fee_collector.wasm") | .code_id' $contracts_file))
  new_code_id=${code_ids[-1]}
  $BINARY tx wasm migrate $fee_collector $new_code_id '{}' $TXFLAG --from $deployer_address

  echo -e "\nMigrating pool factory..."

  code_ids=($(jq -r '.contracts[] | select(.wasm == "terraswap_factory.wasm") | .code_id' $contracts_file))
  new_code_id=${code_ids[-1]}
  $BINARY tx wasm migrate $pool_factory $new_code_id '{}' $TXFLAG --from $deployer_address

  echo -e "\nMigrating pool..."

  pools_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_pools.json
  mapfile pools < <(jq -r '.pools[] | .pool_address' $pools_file)
  pool=$(echo "${pools[0]}" | sed -e 's/\ *$//g')

  code_ids=($(jq -r '.contracts[] | select(.wasm == "terraswap_pair.wasm") | .code_id' $contracts_file))
  new_code_id=${code_ids[-1]}
  migrate_pool_msg='{"migrate_pair":{"contract":"'$pool'","code_id":'$new_code_id'}}'
  $BINARY tx wasm execute $pool_factory "$migrate_pool_msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  echo -e "\nMigrating pool router..."

  code_ids=($(jq -r '.contracts[] | select(.wasm == "terraswap_router.wasm") | .code_id' $contracts_file))
  new_code_id=${code_ids[-1]}
  $BINARY tx wasm migrate $pool_router $new_code_id '{}' $TXFLAG --from $deployer_address

  echo -e "\nMigrating vault factory..."

  code_ids=($(jq -r '.contracts[] | select(.wasm == "vault_factory.wasm") | .code_id' $contracts_file))
  new_code_id=${code_ids[-1]}
  $BINARY tx wasm migrate $vault_factory $new_code_id '{}' $TXFLAG --from $deployer_address

  echo -e "\nMigrating vault..."

  code_ids=($(jq -r '.contracts[] | select(.wasm == "vault.wasm") | .code_id' $contracts_file))
  new_code_id=${code_ids[-1]}
  migrate_vaults_msg='{"migrate_vaults":{"vault_code_id":'$new_code_id'}}'
  $BINARY tx wasm execute $vault_factory "$migrate_vaults_msg" $TXFLAG --from $deployer_address
  sleep $tx_delay

  echo -e "\nMigrating vault router..."

  code_ids=($(jq -r '.contracts[] | select(.wasm == "vault_router.wasm") | .code_id' $contracts_file))
  new_code_id=${code_ids[-1]}
  $BINARY tx wasm migrate $vault_router $new_code_id '{}' $TXFLAG --from $deployer_address
}

function cleanup() {
  # remove pool-test-*.json input files
  rm $project_root_path/scripts/deployment/input/pool-test-*.json
  rm $project_root_path/scripts/deployment/input/vault-test-*.json
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
  migrations)
    test_migrations
    test_liquidity_hub
    ;;
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
