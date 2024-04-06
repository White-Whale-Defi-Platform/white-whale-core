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

while getopts ":d:h:" opt; do
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

# Generates schemas for contracts
cargo xtask generate_schemas

if [[ "$fail_diff_flag" == true ]]; then
	files=$(git ls-files --modified --others --exclude-standard '*.json')

	if [ -n "$files" ]; then
		exit 1
	fi
fi
