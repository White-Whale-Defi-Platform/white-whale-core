#!/usr/bin/env bash

project_root_path=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | cd ../ | pwd)

# Function definition to append, including the load alias
loadenv_function="
#cosmos chains env loader
alias load='load_chain_env'
load_chain_env() {
    if [ -z \"\$1\" ]; then
        echo \"Please specify the chain to load (e.g., migaloo, terra...)\"
    else
        source "${project_root_path}"/scripts/deployment/deploy_env/chain_env.sh
        init_chain_env \"\$1\"
    fi
}
"

# Potential shell configuration files
bashrc="$HOME/.bashrc"
zshrc="$HOME/.zshrc"
profile="$HOME/.profile"

# Append the function to Bash and Zsh configuration files if they exist
[[ -f "$bashrc" ]] && echo "$loadenv_function" >>"$bashrc" && echo "Added to $bashrc"
[[ -f "$zshrc" ]] && echo "$loadenv_function" >>"$zshrc" && echo "Added to $zshrc"
[[ -f "$profile" ]] && echo "$loadenv_function" >>"$profile" && echo "Added to $profile"

echo "Now you can load chains env variables by doing 'load <chain>' in your terminal."
echo "To see a list of compatible chains look into chain_env.sh"
