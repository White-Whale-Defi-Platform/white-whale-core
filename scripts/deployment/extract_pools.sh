#!/usr/bin/env bash
set -e

# Import the deploy_liquidity_hub script
deployment_script_dir=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Displays tool usage
function display_usage() {
	echo "WW Pool Extractor"
	echo -e "\nUsage:./extract_pools.sh [flags].\n"
	echo -e "Available flags:\n"
	echo -e "  -h \thelp"
	echo -e "  -c \tThe chain where you want to extract pools on (juno|juno-testnet|terra|terra-testnet|... check chain_env.sh for the complete list of supported chains)"
}

# Generates the label for the given asset pair
function generate_label() {
	local assets=("$1" "$2")
	local processed_assets=()

	for asset in "${assets[@]}"; do
		# reload the original chain env in case it gets overriden by the cw20:* asset case
		init_chain_env $chain
		local processed_asset="$asset"

		# first of, check if the asset is an ibc token. An IBC token can be a native token on the chain it came from, or a token factory token
		if [[ "$asset" == ibc/* ]]; then
			# do denom trace to get the real denom
			denom_trace_result=$($BINARY q ibc-transfer denom-trace $asset --node $RPC -o json | jq -r '.denom_trace.base_denom')

			# if the denom trace has the pattern factory/* ... This is the factory token model adopted by most token factories
			if [[ "$denom_trace_result" == factory/* ]]; then
				processed_asset=$(echo "$denom_trace_result" | awk -F'/' '{print $NF}')

			# if the denom trace has the pattern factory:*... this is the factory token model adopted by the kujira token factory
			elif [[ "$denom_trace_result" == factory:* ]]; then
				processed_asset="${denom_trace_result##*:}"

			# if the denom trace has the pattern cw20:*...
			elif [[ "$denom_trace_result" == cw20:* ]]; then
				denom_trace_result=${denom_trace_result#*:}

				# remove the cw20: prefix and load the chain env for the chain the token is on
				if [[ $denom_trace_result =~ ([a-zA-Z]+)1 ]]; then
					x=${BASH_REMATCH[1]}
					init_chain_env $x
				else
					continue
				fi

				local query='{"token_info":{}}'
				local symbol=$($BINARY query wasm contract-state smart $denom_trace_result "$query" --node $RPC --output json | jq -r '.data.symbol')
				processed_asset=$symbol
			else # else, just use the denom trace result. This can be the case of native tokens being transferred to another chain, i.e. uwhale on terra
				processed_asset="$denom_trace_result"
			fi

		# Else, check if the asset is a token factory token, i.e. say the asset is ampWHALE on the Migaloo blockchain. The denom will start with factory/*...
		elif [[ "$asset" == factory/* ]]; then
			processed_asset=$(basename "$asset")

		# Else, check if the asset is a cw20 token. This checks if the asset starts with the name of the chain. i.e. terra*, migaloo*...
		elif [[ "$asset" == $chain* ]]; then
			local query='{"token_info":{}}'
			local symbol=$($BINARY query wasm contract-state smart $asset "$query" --node $RPC --output json | jq -r '.data.symbol')
			processed_asset=$symbol
		fi

		# append the $processed_asset into the $processed_assets array
		processed_assets+=("$processed_asset")
	done

	# print the label as asset1_label-asset2_label
	echo "${processed_assets[0]}-${processed_assets[1]}"
}

function extract_asset_info() {
	local asset_json="$1"
	if echo "$asset_json" | jq -e '.native_token' &>/dev/null; then
		echo $(echo "$asset_json" | jq -r '.native_token.denom')
	elif echo "$asset_json" | jq -e '.token' &>/dev/null; then
		echo $(echo "$asset_json" | jq -r '.token.contract_addr')
	else
		echo "unknown"
	fi
}

function extract_pools() {
	mkdir -p $project_root_path/scripts/deployment/output
	output_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_pools.json
	deployment_file=$project_root_path/scripts/deployment/output/"$CHAIN_ID"_liquidity_hub_contracts.json

	if [[ ! -f "$output_file" ]]; then
		# create file to dump results into
		echo '{"pools": []}' | jq '.' >$output_file
	fi

	pool_factory_addr=$(jq -r '.contracts[] | select (.wasm == "terraswap_factory.wasm") | .contract_address' $deployment_file)
	incentive_factory_addr=$(jq -r '.contracts[] | select (.wasm == "incentive_factory.wasm") | .contract_address' $deployment_file)

	local limit=30
	query='{"pairs":{"limit": '$limit'}}'
	local pairs=$($BINARY query wasm contract-state smart $pool_factory_addr "$query" --node $RPC --output json | jq '.data.pairs')

	pool_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_pair.wasm") | .code_id' $deployment_file)
	lp_code_id=$(jq -r '.contracts[] | select (.wasm == "terraswap_token.wasm") | .code_id' $deployment_file)

	echo -e "\nExtracting pools info...\n"
	while IFS= read -r line; do
		# Extracting asset information
		asset1_info=$(extract_asset_info "$(echo $line | jq '.asset_infos[0]')")
		asset2_info=$(extract_asset_info "$(echo $line | jq '.asset_infos[1]')")
		lp_asset=$(echo $line | jq '.liquidity_token')

		# Generate label for the pair
		label=$(generate_label "$asset1_info" "$asset2_info")
		query='{"incentive":{"lp_asset": '$lp_asset'}}'
		local incentive=$($BINARY query wasm contract-state smart $incentive_factory_addr "$query" --node $RPC --output json | jq -r '.data')

		# create the pool entry and append it to the pools array
		pool_entry=$(jq --arg pool_code_id "$pool_code_id" --arg lp_code_id "$lp_code_id" --arg pair_label "$label" --arg incentive "$incentive" '{
            pair: $pair_label,
            assets: .asset_infos,
            pool_address: .contract_addr,
            lp_asset: .liquidity_token,
            incentive_contract: $incentive,
            pool_code_id: $pool_code_id,
            lp_code_id: $lp_code_id
        }' <<<"$line")
		pools+=("$pool_entry")
	done < <(echo "$pairs" | jq -c '.[]')

	# Combine all pool entries into a single JSON array and write to the output file
	jq -n \
		--argjson pools "$(echo "${pools[@]}" | jq -s '.')" \
		--arg pool_factory_addr "$pool_factory_addr" \
		--arg chain "$CHAIN_ID" \
		'{chain: $chain, pool_factory_addr: $pool_factory_addr, pools: $pools }' \
		>"$output_file"

	echo -e "\n**** Extracted pool data on $CHAIN_ID successfully ****\n"
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
		init_chain_env $OPTARG
		extract_pools
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
