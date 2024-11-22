#!/usr/bin/env bash
set -e

projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Displays tool usage
function display_usage() {
	echo "Release builder"
	echo -e "\nUsage:./build_release.sh [flags].\n"
	echo -e "Available flags:\n"
	echo -e "  -c \tThe chain where you want to deploy (migaloo|juno|terra|...)"
}

if [ -z $1 ]; then
	display_usage
	exit 0
fi

while getopts ":c:" opt; do
	case $opt in
	c)
		chain="$OPTARG"
		;;
	\?)
		echo "Invalid option: -$OPTARG" >&2
		display_usage
		exit 1
		;;
	esac
done

flag=""

case $chain in

osmosis)
	flag="-osmosis"
	echo " $projectRootPath/Cargo.toml"

	# backup the Cargo.toml file
	cp $projectRootPath/Cargo.toml $projectRootPath/Cargo.toml.bak

	# add the osmosis feature flag to the Cargo.toml file so it optimizes correctly
	if [[ "$(uname)" == "Darwin" ]]; then
		sed -i '' '/white-whale-std =/ s/}/, features = \["osmosis"\] }/' $projectRootPath/Cargo.toml
	else
		sed -i '/white-whale-std =/ s/}/, features = \["osmosis"\] }/' $projectRootPath/Cargo.toml
	fi

	;;
juno | terra | chihuahua)
	flag="-osmosis_token_factory"
	;;
migaloo)
	flag="-token_factory"
	;;
injective)
	flag="-injective"
	;;
comdex | orai | sei | vanilla | terra-classic) ;;

\*)
	echo "Network $chain not defined"
	exit 1
	;;
esac

projectRootPath=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# if the operative system is running arm64, append -arm64 to workspace-optimizer. Otherwise not
arch=$(uname -m)

docker_options=(
	--rm
	-v "$projectRootPath":/code
	--mount type=volume,source="$(basename "$projectRootPath")_cache",target=/target
	--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry
)

# Make sure you have an image with the flags installed on your docker. For that, fork the rust-optimizer,
# modify the main.rs file adding the feature flag you want to compile with, modify the DOCKER_TAG on the Makefile
# and run make build.

# Optimized builds
if [[ "$arch" == "aarch64" || "$arch" == "arm64" ]]; then
	docker_command=("docker" "run" "${docker_options[@]}" "cosmwasm/optimizer-arm64:0.16.0$flag")
else
	docker_command=("docker" "run" "${docker_options[@]}" "cosmwasm/optimizer:0.16.0$flag")
fi

echo "${docker_command[@]}"

# Execute the Docker command
"${docker_command[@]}"

# Check generated wasm file sizes
$projectRootPath/scripts/check_artifacts_size.sh

# Check generated wasm file sizes
$projectRootPath/scripts/get_artifacts_versions.sh

if [[ "$chain" == "osmosis" ]]; then
	#if the chain is osmosis, restore the Cargo.toml file
	mv $projectRootPath/Cargo.toml.bak $projectRootPath/Cargo.toml
fi
