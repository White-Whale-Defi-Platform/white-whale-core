# Prints the list of recipes.
default:
    @just --list

# Builds the whole project with the a feature flag if provided.
build FEATURE='':
  #!/usr/bin/env sh
  echo "-- Building {{FEATURE}} -- \n"
  if [ -z "{{FEATURE}}" ]; then
    cargo build
  else
    cargo build --features {{FEATURE}}
  fi

# Build all schemas
schemas:
  scripts/build_schemas.sh

# Tests the whole project with the a feature flag if provided.
test FEATURE='':
  #!/usr/bin/env sh
  if [ -z "{{FEATURE}}" ]; then
    cargo test
  else
    cargo test --features {{FEATURE}}
  fi

# Alias to the format recipe.
fmt:
  @just format

# Formats the rust, toml and sh files in the project.
format:
  cargo fmt --all
  find . -type f -iname "*.toml" -print0 | xargs -0 taplo format
  find . -type f -name '*.sh' -exec shfmt -w {} \;

# Runs clippy with the a feature flag if provided.
lint FEATURE='':
  #!/usr/bin/env sh
  if [ -z "{{FEATURE}}" ]; then
    cargo clippy --all -- -D warnings
  else
    cargo clippy --features {{FEATURE}} --all -- -D warnings
  fi

# Tries to fix clippy issues automatically.
lintfix:
  cargo clippy --fix --allow-staged --allow-dirty --all-features
  just format

# Checks the whole project with all the feature flags.
check-all:
  cargo check --all-features

# Cargo check.
check:
  cargo check

# Cargo clean and update.
refresh:
  cargo clean && cargo update

# Cargo watch.
watch:
  cargo watch -x lcheck

# Watches tests with the a feature flag if provided.
watch-test FEATURE='':
  #!/usr/bin/env sh
  if [ -z "{{FEATURE}}" ]; then
    cargo watch -x "nextest run"
  else
    cargo watch -x "nextest run --features {{FEATURE}}"
  fi

# Compiles and optimizes the contracts for the specified chain.
optimize CHAIN:
  scripts/build_release.sh -c {{CHAIN}}

# Prints the artifacts versions on the current commit.
get-artifacts-versions:
  scripts/get_artifacts_versions.sh

# Prints the artifacts size. Optimize should be called before.
get-artifacts-size:
  scripts/check_artifacts_size.sh

# Extracts the pools from the given chain.
get-pools CHAIN:
    scripts/deployment/extract_pools.sh -c {{CHAIN}}

# Installs the env loader locally.
install-env-loader:
    scripts/deployment/deploy_env/add_load_chain_env_alias.sh

# Deploys the contracts to the specified chain.
deploy CHAIN ARTIFACT='all':
  scripts/deployment/deploy_liquidity_hub.sh -c {{CHAIN}} -d {{ARTIFACT}}

# Stores the contracts to the specified chain.
store CHAIN ARTIFACT='all':
  scripts/deployment/deploy_liquidity_hub.sh -c {{CHAIN}} -s {{ARTIFACT}}

# Migrates the contracts to the specified chain.
migrate CHAIN ARTIFACT='all':
  scripts/deployment/migrate_liquidity_hub.sh -c {{CHAIN}} -m {{ARTIFACT}}
