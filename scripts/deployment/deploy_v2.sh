#!/usr/bin/env bash
set -e
#set -x

deployment_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)
artifacts_path=$project_root_path/artifacts

source $deployment_script_dir/ibc_denoms.sh

# Displays tool usage
function display_usage() {
	echo "WW V2 Deployer"
	echo -e "\nUsage:./deploy_v2.sh [flags]. Two flags should be used, -c to specify the chain and then either -d or -s."
	echo -e "To deploy V2, the contracts need to be stored first, running -s. With the code_ids in place, the contracts can be deployed with -d.\n"
	echo -e "Available flags:\n"
	echo -e "  -h \thelp"
	echo -e "  -c \tThe chain where you want to deploy (juno|juno-testnet|terra|terra-testnet|... check chain_env.sh for the complete list of supported chains)"
	echo -e "  -d \tWhat to deploy (all|pool-manager|vault-manager|epoch-manager|bonding-manager|incentive-manager)"
	echo -e "  -s \tStore artifacts on chain (all|pool-manager|vault-manager|epoch-manager|bonding-manager|incentive-manager)"
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

function init_epoch_manager() {
	init_msg='{
    "start_epoch": {
      "id": 0,
      "start_time": "1571797419879305533"
    },
    "epoch_config": {
      "duration": "86400000000000",
      "genesis_epoch": "1571797419879305533"
    }
  }'
	init_artifact 'epoch_manager.wasm' "$init_msg" "White Whale Epoch Manager"
}

function init_pool_manager() {
	bonding_manager_addr=$(jq '.contracts[] | select (.wasm == "bonding_manager.wasm") | .contract_address' $output_file)
	incentive_manager_addr=$(jq '.contracts[] | select (.wasm == "incentive_manager.wasm") | .contract_address' $output_file)

	init_msg='{
              "bonding_manager_addr": "'$bonding_manager_addr'",
              "incentive_manager_addr": "'$incentive_manager_addr'",
              "pool_creation_fee": {
                "denom": "uwhale",
                "amount": "1000000000"
              }
            }'
	init_artifact 'pool_manager.wasm' "$init_msg" "White Whale Pool Manager"
}

function init_vault_manager() {
	bonding_manager_addr=$(jq '.contracts[] | select (.wasm == "bonding_manager.wasm") | .contract_address' $output_file)

	init_msg='{
              "owner": "migaloo1...",
              "bonding_manager_addr": "'$bonding_manager_addr'",
              "vault_creation_fee": {
                "denom": "uwhale",
                "amount": "1000"
              }
            }'
	init_artifact 'vault_manager.wasm' "$init_msg" "White Whale Vault Manager"
}

function init_bonding_manager() {
	epoch_manager_addr=$(jq '.contracts[] | select (.wasm == "epoch_manager.wasm") | .contract_address' $output_file)

	init_msg='{
              "distribution_denom": "uwhale",
              "unbonding_period": 1,
              "growth_rate": "0.1",
              "bonding_assets": [
                "ampWHALE",
                "bWHALE"
              ],
              "grace_period": 21,
              "epoch_manager_addr": "'$epoch_manager_addr'"
            }'
	init_artifact 'bonding_manager.wasm' "$init_msg" "White Whale Bonding Manager"
}

function init_incentive_manager() {
	epoch_manager_addr=$(jq '.contracts[] | select (.wasm == "epoch_manager.wasm") | .contract_address' $output_file)
	bonding_manager_addr=$(jq '.contracts[] | select (.wasm == "bonding_manager.wasm") | .contract_address' $output_file)

	init_msg='{
              "owner": "migaloo1...",
              "epoch_manager_addr": "'$epoch_manager_addr'",
              "bonding_manager_addr": "'$bonding_manager_addr'",
              "create_incentive_fee": {
                "denom": "uwhale",
                "amount": "1000000000"
              },
              "max_concurrent_incentives": 7,
              "max_incentive_epoch_buffer": 14,
              "min_unlocking_duration": 86400,
              "max_unlocking_duration": 31536000,
              "emergency_unlock_penalty": "0.01"
            }'
	init_artifact 'incentive_manager.wasm' "$init_msg" "White Whale Incentive Manager"
}

function init_v2() {
	echo -e "\nInitializing WW V2 on $CHAIN_ID..."

	init_epoch_manager
	init_bonding_manager
	init_incentive_manager
	init_pool_manager
	init_vault_manager
}

function init_artifact() {
	if [ $# -eq 3 ]; then
		local artifact=$1
		local init_msg=$2
		local label=$3
	else
		echo "init_artifact needs the artifact, init_msg and label"
		exit 1
	fi

	echo -e "\nInitializing $artifact on $CHAIN_ID..."

	# Instantiate the contract
	code_id=$(jq -r '.contracts[] | select (.wasm == "'$artifact'") | .code_id' $output_file)
	$BINARY tx wasm instantiate $code_id "$init_msg" --from $deployer --label "$label" $TXFLAG --admin $deployer_address
	sleep $tx_delay
	# Get contract address
	contract_address=$($BINARY query wasm list-contract-by-code $code_id --node $RPC --output json | jq -r '.contracts[-1]')

	# Append contract_address to output file
	append_contract_address_to_output $contract_address $artifact
	sleep $tx_delay
}

function deploy() {
	mkdir -p $project_root_path/scripts/deployment/output
	output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_v2_contracts.json

	if [[ ! -f "$output_file" ]]; then
		# create file to dump results into
		echo '{"contracts": []}' | jq '.' >$output_file
		initial_block_height=$(curl -s $RPC/abci_info? | jq -r '.result.response.last_block_height')
	else
		# read from existing deployment file
		initial_block_height=$(jq -r '.initial_block_height' $output_file)
	fi

	echo -e "\e[1;31m⚠️ WARNING ⚠️️\e[0m"

	echo -e "\e[1;32mThis script assumes the init messages for each contract have been adjusted to your likes.\e[0m"
	echo -e "\n\e[1;32mIf that is not the case, please abort the deployment and make the necessary changes, then run the script again :)\e[0m"

	echo -e "\nDo you want to proceed? (y/n)"
	read proceed

	if [[ "$proceed" != "y" ]]; then
		echo "Deployment cancelled..."
		exit 1
	fi

	case $1 in
	epoch-manager)
		init_epoch_manager
		;;
	pool-manager)
		init_pool_manager
		;;
	vault-manager)
		init_vault_manager
		;;
	bonding-manager)
		init_bonding_manager
		;;
	incentive-manager)
		init_incentive_manager
		;;
	*) # store all
		init_v2
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
	output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_v2_contracts.json

	if [[ ! -f "$output_file" ]]; then
		# create file to dump results into
		echo '{"contracts": []}' | jq '.' >$output_file
	fi

	case $1 in
	epoch-manager)
		store_artifact_on_chain $artifacts_path/epoch_manager.wasm
		;;
	pool-manager)
		store_artifact_on_chain $artifacts_path/pool_manager.wasm
		;;
	vault-manager)
		store_artifact_on_chain $artifacts_path/vault_manager.wasm
		;;
	bonding-manager)
		store_artifact_on_chain $artifacts_path/bonding_manager.wasm
		;;
	incentive-manager)
		store_artifact_on_chain $artifacts_path/incentive_manager.wasm
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
