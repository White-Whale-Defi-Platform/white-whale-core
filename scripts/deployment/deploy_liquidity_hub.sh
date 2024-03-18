#!/usr/bin/env bash
set -e
#set -x

deployment_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)
artifacts_path=$project_root_path/artifacts

source $deployment_script_dir/ibc_denoms.sh

# Displays tool usage
function display_usage() {
	echo "WW Liquidity Hub Deployer"
	echo -e "\nUsage:./deploy_liquidity_hub.sh [flags]. Two flags should be used, -c to specify the chain and then either -d or -s."
	echo -e "To deploy a new hub the contracts need to be stored first, running -s. With the code_ids in place, the contracts can be deployed with -d.\n"
	echo -e "Available flags:\n"
	echo -e "  -h \thelp"
	echo -e "  -c \tThe chain where you want to deploy (juno|juno-testnet|terra|terra-testnet|... check chain_env.sh for the complete list of supported chains)"
	echo -e "  -d \tWhat to deploy (all|pool-network|vault-network|fee-collector|pool-factory|pool-router|vault-factory|vault-router|fee-distributor|whale-lair)"
	echo -e "  -s \tStore artifacts on chain (all|fee-collector|pool-factory|pool|token|pool-router|vault|vault-factory|vault-router|fee-distributor|whale-lair)"
	echo -e "  -a \tArtifacts folder path (default: $project_root_path/artifacts)"
}

function store_artifact_on_chain() {
	if [ $# -eq 1 ]; then
		local artifact=$1
	else
		echo "store_artifact_on_chain needs the artifact path"
		exit 1
	fi

	echo "Storing $(basename $artifact) on $CHAIN_ID..."

	# Get contract version for storing purposes
	local contract_path=$(find "$project_root_path" -iname $(cut -d . -f 1 <<<$(basename $artifact)) -type d)
	local version=$(cat ''"$contract_path"'/Cargo.toml' | awk -F= '/^version/ { print $2 }')
	local version="${version//\"/}"

	local res=$($BINARY tx wasm store $artifact $TXFLAG --from $deployer | jq -r '.txhash')
	sleep $tx_delay
	local code_id=$($BINARY q tx $res --node $RPC -o json | jq -r '.logs[0].events[] | select(.type == "store_code").attributes[] | select(.key == "code_id").value')

	# Download the wasm binary from the chain and compare it to the original one
	echo -e "Verifying integrity of wasm artifact on chain...\n"
	$BINARY query wasm code $code_id --node $RPC downloaded_wasm.wasm >/dev/null 2>&1
	# The two binaries should be identical
	diff $artifact downloaded_wasm.wasm
	rm downloaded_wasm.wasm

	# Write code_id in output file
	tmpfile=$(mktemp)
	jq --arg artifact "$(basename "$artifact")" --arg code_id "$code_id" --arg version "$version" '.contracts += [{"wasm": $artifact, "code_id": $code_id, "version": $version}]' "$output_file" >"$tmpfile"
	mv $tmpfile $output_file
	echo -e "Stored artifact $(basename "$artifact") on $CHAIN_ID successfully\n"
	sleep $tx_delay
}

function store_artifacts_on_chain() {
	for artifact in $artifacts_path/*.wasm; do
		store_artifact_on_chain $artifact
	done

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
	jq -r --arg contract_address $contract_address --arg wasm_file_name $wasm_file_name '.contracts[] | select (.wasm == $wasm_file_name) |= . + {contract_address: $contract_address}' $output_file | jq -n '.contracts |= [inputs]' >$tmpfile
	mv $tmpfile $output_file
}

function init_fee_collector() {
	echo -e "\nInitializing the Fee Collector..."

	# Prepare the instantiation message
	init='{}'
	# Instantiate the contract
	code_id=$(jq -r '.contracts[] | select (.wasm == "fee_collector.wasm") | .code_id' $output_file)
	$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Fee Collector" $TXFLAG --admin $deployer_address
	sleep $tx_delay
	# Get contract address
	contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

	# Append contract_address to output file
	append_contract_address_to_output $contract_address 'fee_collector.wasm'
	sleep $tx_delay
}

function init_fee_distributor() {
	echo -e "\nInitializing the Fee Distributor..."

	whale_denom=$(extract_ibc_denom "$CHAIN_ID")

	# Prepare the instantiation message
	bonding_contract_addr=$(jq '.contracts[] | select (.wasm == "whale_lair.wasm") | .contract_address' $output_file)
	fee_collector_addr=$(jq '.contracts[] | select (.wasm == "fee_collector.wasm") | .contract_address' $output_file)
	grace_period=21 #default value is 21 epochs
	distribution_asset='{"native_token":{"denom":"'$whale_denom'"}}'
	epoch_duration=86400000000000     #default value is 1 day, in nanoseconds
	genesis_epoch=1706540400000000000 #fill with desired unix time, in nanoseconds
	epoch_config='{"duration":"'$epoch_duration'", "genesis_epoch": "'$genesis_epoch'"}'

	init='{"bonding_contract_addr": '"$bonding_contract_addr"', "fee_collector_addr": '"$fee_collector_addr"', "grace_period": "'$grace_period'", "epoch_config": '"$epoch_config"', "distribution_asset": '"$distribution_asset"'}'

	# Instantiate the contract
	code_id=$(jq -r '.contracts[] | select (.wasm == "fee_distributor.wasm") | .code_id' $output_file)
	$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Fee Distributor" $TXFLAG --admin $deployer_address
	sleep $tx_delay
	# Get contract address
	contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

	# Append contract_address to output file
	append_contract_address_to_output $contract_address 'fee_distributor.wasm'
	sleep $tx_delay
}

function init_whale_lair() {
	echo -e "\nInitializing the Whale Lair..."

	# Prepare the instantiation message
	unbonding_period=86400000000000    # default value is 14 days, in nanoseconds
	growth_rate="0.000000064300411522" # this is the value when you interpolate the growth rate to 2X with 365 days of bonding
	bonding_assets='[
                   {"native_token": {"denom": "uwhale"}},
                   {"native_token": {"denom": "factory/migaloo1dpx7ytug647wefe7ajxmg5ejt68gxcfvw35f4e/test"}}
                 ]'

	init="{\"unbonding_period\": \"$unbonding_period\", \"growth_rate\": \"$growth_rate\", \"bonding_assets\": $bonding_assets}"

	# Instantiate the contract
	code_id=$(jq -r '.contracts[] | select (.wasm == "whale_lair.wasm") | .code_id' $output_file)
	$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Lair" $TXFLAG --admin $deployer_address
	sleep $tx_delay
	# Get contract address
	contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

	# Append contract_address to output file
	append_contract_address_to_output $contract_address 'whale_lair.wasm'
	sleep $tx_delay
}

function init_incentive_factory() {
	echo -e "\nInitializing the Incentive Factory..."

	# Prepare the instantiation message
	incentive_code_id=$(jq -r '.contracts[] | select (.wasm == "incentive.wasm") | .code_id' $output_file)
	fee_distributor_addr=$(jq '.contracts[] | select (.wasm == "fee_distributor.wasm") | .contract_address' $output_file)
	fee_collector_addr=$(jq '.contracts[] | select (.wasm == "fee_collector.wasm") | .contract_address' $output_file)
	whale_denom=$(extract_ibc_denom "$CHAIN_ID")
	create_flow_fee='{"amount": "1000000000", "info": {"native_token":{"denom":"'$whale_denom'"}}}'
	max_concurrent_flows=5          #we start with 5 concurrent flows
	max_flow_epoch_buffer=14        #default value is 14 epochs
	min_unbonding_duration=86400    #default value is 1 day, in seconds
	max_unbonding_duration=31556926 #default value is 1 year, in seconds

	init=$(jq -n \
		--arg fee_collector_addr "$fee_collector_addr" \
		--arg fee_distributor_addr "$fee_distributor_addr" \
		--arg create_flow_fee "$create_flow_fee" \
		--argjson max_concurrent_flows "$max_concurrent_flows" \
		--argjson incentive_code_id "$incentive_code_id" \
		--argjson max_flow_epoch_buffer "$max_flow_epoch_buffer" \
		--argjson min_unbonding_duration "$min_unbonding_duration" \
		--argjson max_unbonding_duration "$max_unbonding_duration" \
		'{ "fee_collector_addr": $fee_collector_addr[1:-1], "fee_distributor_addr": $fee_distributor_addr[1:-1], "create_flow_fee": ($create_flow_fee | fromjson), "max_concurrent_flows": $max_concurrent_flows, "incentive_code_id": $incentive_code_id, "max_flow_epoch_buffer": $max_flow_epoch_buffer, "min_unbonding_duration": $min_unbonding_duration, "max_unbonding_duration": $max_unbonding_duration }')

	# Instantiate the contract
	code_id=$(jq -r '.contracts[] | select (.wasm == "incentive_factory.wasm") | .code_id' $output_file)
	$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Incentive Factory" $TXFLAG --admin $deployer_address
	sleep $tx_delay
	# Get contract address
	contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

	# Append contract_address to output file
	append_contract_address_to_output $contract_address 'incentive_factory.wasm'
	sleep $tx_delay
}

function init_frontend_helper() {
	echo -e "\nInitializing the Frontend Helper..."

	# Prepare the instantiation message
	incentive_factory=$(jq '.contracts[] | select (.wasm == "incentive_factory.wasm") | .contract_address' $output_file)

	init='{"incentive_factory": '"$incentive_factory"'}'

	# Instantiate the contract
	code_id=$(jq -r '.contracts[] | select (.wasm == "frontend_helper.wasm") | .code_id' $output_file)
	$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Frontend Helper" $TXFLAG --admin $deployer_address
	sleep $tx_delay
	# Get contract address
	contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

	# Append contract_address to output file
	append_contract_address_to_output $contract_address 'frontend_helper.wasm'
	sleep $tx_delay
}

function init_pool_factory() {
	echo -e "\nInitializing the Pool Factory..."

	# Prepare the instantiation message
	pair_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_pair.wasm") | .code_id' $output_file)
	trio_code_id=$(jq -r '.contracts[] | select (.wasm == "stableswap_3pool.wasm") | .code_id' $output_file)
	token_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_token.wasm") | .code_id' $output_file)
	fee_collector_addr=$(jq '.contracts[] | select (.wasm == "fee_collector.wasm") | .contract_address' $output_file)

	init='{"pair_code_id": '"$pair_code_id"', "trio_code_id": '"$trio_code_id"', "token_code_id": '"$token_code_id"', "fee_collector_addr": '"$fee_collector_addr"'}'

	# Instantiate the contract
	code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .code_id' $output_file)
	$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Pool Factory" $TXFLAG --admin $deployer_address
	sleep $tx_delay
	# Get contract address
	contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

	# Append contract_address to output file
	append_contract_address_to_output $contract_address 'terraswap_factory.wasm'
	sleep $tx_delay
}

function init_pool_router() {
	echo -e "\nInitializing the Pool Router..."

	# Prepare the instantiation message
	terraswap_factory=$(jq '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $output_file)

	init='{"terraswap_factory": '"$terraswap_factory"'}'
	# Instantiate the contract
	code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_router.wasm") | .code_id' $output_file)
	$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Pool Router" $TXFLAG --admin $deployer_address
	sleep $tx_delay
	# Get contract address
	contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

	# Append contract_address to output file
	append_contract_address_to_output $contract_address 'terraswap_router.wasm'
	sleep $tx_delay
}

function init_vault_factory() {
	echo -e "\nInitializing the Vault Factory..."

	# Prepare the instantiation message
	vault_id=$(jq -r '.contracts[] | select (.wasm == "vault.wasm") | .code_id' $output_file)
	token_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_token.wasm") | .code_id' $output_file)
	fee_collector_addr=$(jq '.contracts[] | select (.wasm == "fee_collector.wasm") | .contract_address' $output_file)

	init='{"owner": "'$deployer_address'", "vault_id": '"$vault_id"', "token_id": '"$token_code_id"', "fee_collector_addr": '"$fee_collector_addr"'}'

	# Instantiate the contract
	code_id=$(jq -r '.contracts[] | select (.wasm == "vault_factory.wasm") | .code_id' $output_file)
	$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Vault Factory" $TXFLAG --admin $deployer_address
	sleep $tx_delay
	# Get contract address
	contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

	# Append contract_address to output file
	append_contract_address_to_output $contract_address 'vault_factory.wasm'
	sleep $tx_delay
}

function init_vault_router() {
	echo -e "\nInitializing the Vault Router..."

	# Prepare the instantiation message
	vault_factory_addr=$(jq '.contracts[] | select (.wasm == "vault_factory.wasm") | .contract_address' $output_file)

	init='{"owner": "'$deployer_address'", "vault_factory_addr": '"$vault_factory_addr"'}'

	# Instantiate the contract
	code_id=$(jq -r '.contracts[] | select (.wasm == "vault_router.wasm") | .code_id' $output_file)
	$BINARY tx wasm instantiate $code_id "$init" --from $deployer --label "White Whale Vault Router" $TXFLAG --admin $deployer_address
	sleep $tx_delay
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
	init_whale_lair
	init_fee_distributor
	init_vault_network
	init_incentive_factory
	init_frontend_helper
}

function deploy() {
	mkdir -p $project_root_path/scripts/deployment/output
	output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json

	if [[ ! -f "$output_file" ]]; then
		# create file to dump results into
		echo '{"contracts": []}' | jq '.' >$output_file
		initial_block_height=$(curl -s $RPC/abci_info? | jq -r '.result.response.last_block_height')
	else
		# read from existing deployment file
		initial_block_height=$(jq -r '.initial_block_height' $output_file)
	fi

	case $1 in
	pool-network)
		init_pool_network
		;;
	vault-network)
		init_vault_network
		;;
	fee-collector)
		init_fee_collector
		;;
	fee-distributor)
		init_fee_distributor
		;;
	incentive-factory)
		init_incentive_factory
		;;
	whale-lair)
		init_whale_lair
		;;
	frontend-helper)
		init_frontend_helper
		;;
	pool-factory)
		init_pool_factory
		;;
	pool-router)
		init_pool_router
		;;
	vault-factory)
		init_vault_factory
		;;
	vault-router)
		init_vault_router
		;;
	*) # deploy all
		init_liquidity_hub
		;;
	esac

	final_block_height=$(curl -s $RPC/abci_info? | jq -r '.result.response.last_block_height')

	# Add additional deployment information
	date=$(date -u +"%Y-%m-%dT%H:%M:%S%z")
	tmpfile=$(mktemp)
	jq --arg date $date --arg chain_id $CHAIN_ID --arg deployer_address $deployer_address --arg initial_block_height $initial_block_height --arg final_block_height $final_block_height '. + {date: $date , initial_block_height: $initial_block_height, final_block_height: $final_block_height, chain_id: $chain_id, deployer_address: $deployer_address}' $output_file >$tmpfile
	mv $tmpfile $output_file

	echo -e "\n**** Deployment successful ****\n"
	jq '.' $output_file
}

function store() {
	mkdir -p $project_root_path/scripts/deployment/output
	output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json

	if [[ ! -f "$output_file" ]]; then
		# create file to dump results into
		echo '{"contracts": []}' | jq '.' >$output_file
	fi

	case $1 in
	fee-collector)
		store_artifact_on_chain $artifacts_path/fee_collector.wasm
		;;
	fee-distributor)
		store_artifact_on_chain $artifacts_path/fee_distributor.wasm
		;;
	pool-factory)
		store_artifact_on_chain $artifacts_path/terraswap_factory.wasm
		;;
	pool)
		store_artifact_on_chain $artifacts_path/terraswap_pair.wasm
		;;
	token)
		store_artifact_on_chain $artifacts_path/terraswap_token.wasm
		;;
	pool-router)
		store_artifact_on_chain $artifacts_path/terraswap_router.wasm
		;;
	vault)
		store_artifact_on_chain $artifacts_path/vault.wasm
		;;
	vault-factory)
		store_artifact_on_chain $artifacts_path/vault_factory.wasm
		;;
	vault-router)
		store_artifact_on_chain $artifacts_path/vault_router.wasm
		;;
	whale-lair)
		store_artifact_on_chain $artifacts_path/whale_lair.wasm
		;;
	incentive)
		store_artifact_on_chain $artifacts_path/incentive.wasm
		;;
	incentive-factory)
		store_artifact_on_chain $artifacts_path/incentive_factory.wasm
		;;
	frontend-helper)
		store_artifact_on_chain $artifacts_path/frontend_helper.wasm
		;;
	*) # store all
		store_artifacts_on_chain
		;;
	esac
}

if [ -z $1 ]; then
	display_usage
	exit 0
fi

# get args
optstring=':c:d:s:a:h'
while getopts $optstring arg; do
	source $deployment_script_dir/wallet_importer.sh

	case "$arg" in
	c)
		chain=$OPTARG
		source $deployment_script_dir/deploy_env/chain_env.sh
		init_chain_env $OPTARG
		if [[ "$chain" = "local" ]]; then
			tx_delay=0.5
		else
			tx_delay=8
		fi
		;;
	d)
		import_deployer_wallet $chain
		deploy $OPTARG
		;;
	s)
		import_deployer_wallet $chain
		store $OPTARG
		;;
	a)
		artifacts_path=$OPTARG
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
