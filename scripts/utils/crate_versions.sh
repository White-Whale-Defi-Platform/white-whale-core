#!/bin/bash

project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)
cargo_lock_file="$project_root_path/Cargo.lock"

# Extracts crate names and versions
extract_crates() {
  cargo_lock_file="$1"
  filter_mode="$2"

  # Extract crate names and versions using awk
  awk -F '"' '/^name = / { name=$2 } /^version = / { version=$2 } name && version { printf "%s (%s)\n", name, version; name=""; version="" }' "$cargo_lock_file" |
    filter_crates "$filter_mode"
}

# Filters crate names based on a preset array
filter_crates() {
  filter_mode="$1"

  if [ "$filter_mode" = "all" ]; then
    # No filtering, pass through all crate names
    cat -
  else
    # Define the preset array of crate names to filter, i.e. white whale contracts
    declare -a crate_names=("fee_collector" "fee_distributor" "frontend-helper" "incentive" "incentive-factory" "stableswap-3pool" "terraswap-factory" "terraswap-pair" "terraswap-router" "terraswap-token" "vault" "vault_factory" "vault_router" "whale-lair")

    # Filter the crate names based on the preset array
    grep -E "($(IFS="|"; echo "${crate_names[*]}"))"
  fi
}

if [ "$1" = "-h" ]; then
    echo -e "\nUsage: crate_versions.sh [OPTION]\n"
    echo "List names and versions of Cargo crates in White Whale Core."
    echo "If no option is provided, only White Whale contracts and versions will be shown."
    echo ""
    echo "Options:"
    echo "-a        Show all crate names and versions"
    echo -e "-h        Show this help message\n"
    exit 0
fi

cargo build

filter_mode="all"

# Check if the -a flag is provided
if [ "$1" = "-a" ]; then
  filter_mode="all"
else
  filter_mode="white-whale"
fi

# Call the extract_crates function with the filter mode
crates=$(extract_crates "$cargo_lock_file" "$filter_mode")

if [ "$filter_mode" = "all" ]; then
    echo -e "\nAll crate names and versions:\n"
else
    echo -e "\nWhite Whale contracts and versions:\n"
fi
echo -e "$crates\n"
