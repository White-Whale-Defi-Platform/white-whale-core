#!/usr/bin/env bash
set -e

# Displays tool usage
function display_usage() {
	echo "Schema generator"
	echo -e "\nUsage: $0 [flags].\n"
	echo -e "Available flags:\n"
	echo -e "  -f \tSpecify the feature to use (token_factory|osmosis_token_factory|...)"
	echo -e "  -d \tEnable diff check (true|false)"
}

if [ -z $1 ]; then
	display_usage
	exit 0
fi

feature_flag=""
fail_diff_flag=false

while getopts ":f:d:" opt; do
	case $opt in
	f)
		feature_flag="$OPTARG"
		;;
	d)
		fail_diff_flag="$OPTARG"
		;;
	\?)
		echo "Invalid option: -$OPTARG" >&2
		display_usage
		exit 1
		;;
	esac
done

# First argument, whether or not to run git diff and exit with an error on any json file diff, not used by default
if [[ -z $1 ]]; then
	fail_diff_flag=false
else
	fail_diff_flag=$1
fi

projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Generates schemas for contracts in the liquidity_hub
for component in "$projectRootPath"/contracts/liquidity_hub/*/; do
	echo "Generating schemas for $(basename $component)..."
	if [[ -f "$component/Cargo.toml" ]]; then
		# it was a single contract (such as fee_collector)
		echo "generating for $component"
		cd $component && cargo schema --locked
	else
		echo "folder $component"

		# it was a directory (such as pool_network), do it for all files inside the directory
		for contract in "$component"*/; do
			echo "generating for $contract"

			cd $contract && cargo schema --locked

			# Optionally fail on any unaccounted changes in json schema files
			if [[ "$fail_diff_flag" == true ]]; then
				git diff --exit-code -- '*.json'
			fi
		done
	fi
done
