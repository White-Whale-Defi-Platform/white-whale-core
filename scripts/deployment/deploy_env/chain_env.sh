#!/bin/bash
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

  case $chain in

  local)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/local.env)
    ;;

  juno)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/mainnets/juno.env)
    ;;

  juno-testnet)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/juno.env)
    ;;

  terra)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/mainnets/terra.env)
    ;;

  terra-testnet)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/terra.env)
    ;;

  archway-testnet)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/archway.env)
    ;;

  chihuahua)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/mainnets/chihuahua.env)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/base_chihuahua.env)
    ;;

  injective)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/mainnets/injective.env)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/base_injective.env)
    ;;

  injective-testnet)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/injective.env)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/base_injective.env)
    ;;

  comdex)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/mainnets/comdex.env)
    ;;

  comdex-testnet)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/comdex.env)
    ;;

  sei-testnet)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/sei.env)
    ;;

  stargaze-testnet)
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/testnets/stargaze.env)
    ;;

  *)
    echo "Network $chain not defined"
    exit 1
    ;;
  esac

  if [[ $chain != "chihuahua" && $chain != "injective" && $chain != "injective-testnet" ]]; then
    source <(cat "$project_root_path"/scripts/deployment/deploy_env/base.env)
  fi
}
