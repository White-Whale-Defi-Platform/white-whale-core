#!/usr/bin/env bash
set -e

project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

echo -e "\nGetting artifacts versions...\n"
echo -e "Contracts::\n"
for artifact in artifacts/*.wasm; do
  artifact="${artifact%-*}"
  contract_path=$(find "$project_root_path" -iname $(cut -d . -f 1 <<<$(basename $artifact)) -type d)
  version=$(cat ''"$contract_path"'/Cargo.toml' | awk -F= '/^version/ { print $2 }')
  version="${version//\"/}"

  printf "%-25s %s\n" "$(basename $artifact)" ": $version"
done
echo -e "\nPackages::\n"

version=$(grep 'white-whale-std = ' ''"$project_root_path"'/Cargo.toml' | sed -n 's/.*version = "\([^"]*\)".*/\1/p')
printf "%-25s %s\n" "white-whale-std" ":  $version"

echo -e "\n"
