#!/usr/bin/env bash
set -e

project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Initializes chain env variables
function init_chain_env() {
	if [ $# -eq 1 ]; then
		local chain=$1
	else
		echo "init_chain_env requires a chain"
		exit 1
	fi

    # Define the environment type based on the chain argument
    local env_type="mainnets"
    if [[ $chain == *"-testnet" ]]; then
        env_type="testnets"
    fi

	source <(cat "$project_root_path"/scripts/deployment/deploy_env/"$env_type"/"${chain%-testnet}.env")

	if [[ $chain != "chihuahua" && $chain != "injective" && $chain != "injective-testnet" && $chain != "migaloo" && $chain != "migaloo-testnet" ]]; then
		source <(cat "$project_root_path"/scripts/deployment/deploy_env/base.env)
	fi
}

init_chain_env $1
