#!/usr/bin/env bash

# The denoms file in White Whale docs.
# The table looks like this -> | Name | Chain-ID | Denom | Logo |
github_denoms_link="https://raw.githubusercontent.com/White-Whale-Defi-Platform/white-whale-docs/main/gitbook/smart-contracts/assets/denoms.md"

# Function to extract the whale IBC denom based on chain ID.
# On migaloo, whale is the native token so the denom is uwhale.
# Otherwise, it will be an ibc/denom.
extract_ibc_denom() {
	chain_id="$1"

	# Download the markdown file
	curl -s "$github_denoms_link" -o .denom_temp.md

	# Extract the denom based on the chain ID
	denom=$(awk -F '|' -v chain_id="$chain_id" '$3 ~ chain_id {gsub(/^[ \t]+| [ \t]+$/, "", $4); print $4}' .denom_temp.md)

	# Clean up temporary file
	rm .denom_temp.md

	echo $(echo "$denom" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')
}
