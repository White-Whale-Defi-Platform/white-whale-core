#!/usr/bin/env bash
set -e

# Displays tool usage
function display_usage() {
	echo "Schema generator"
	echo -e "\nUsage: $0 [flags].\n"
	echo -e "Available flags:\n"
	echo -e "  -d \tEnable diff check (true|false), defaults to false"
	echo -e "  -h \tDisplay this help menu"
}

fail_diff_flag=false

while getopts ":f:d:" opt; do
	case $opt in
	d)
		fail_diff_flag="$OPTARG"
		;;
	h)
		display_usage
		exit 0
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

# Generates schemas for contracts
cargo xtask generate_schemas

if [[ "$fail_diff_flag" == true ]]; then
	git diff --exit-code -- '*.json'
fi
