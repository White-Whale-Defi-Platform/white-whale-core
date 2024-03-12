#!/usr/bin/env bash
#set -e

project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Initializes chain env variables
function init_chain_env() {
	if [ $# -eq 1 ]; then
		local chain=$1
	else
		echo "init_chain_env requires a chain"
		exit 1
	fi

	if [[ "$(echo ${chain##*-})" = "testnet" ]] || [[ "$chain" = "local" ]]; then
		chain="${chain%-testnet}"
		source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/"$chain".env)
	else
		source <(cat "$project_root_path"/scripts/deployment/deploy_env/mainnets/"$chain".env)
	fi

	# load the base env, i.e. the TXFLAG
	source "$project_root_path"/scripts/deployment/deploy_env/base_env.sh $chain
}
