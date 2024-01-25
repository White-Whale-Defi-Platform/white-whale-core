build FEATURE='':
  #!/usr/bin/env sh
  echo "-- Building {{FEATURE}} -- \n"
  if [ -z "{{FEATURE}}" ]; then
    cargo build
  else
    cargo build --features {{FEATURE}}
  fi

schemas:
  scripts/build_schemas.sh

test FEATURE='':
  #!/usr/bin/env sh
  if [ -z "{{FEATURE}}" ]; then
    cargo test
  else
    cargo test --features {{FEATURE}}
  fi

format:
  cargo fmt --all
  find . -type f -iname "*.toml" -print0 | xargs -0 taplo format

lint FEATURE='':
  #!/usr/bin/env sh
  if [ -z "{{FEATURE}}" ]; then
    cargo clippy --all -- -D warnings
  else
    cargo clippy --features {{FEATURE}} --all -- -D warnings
  fi

lintfix:
  cargo clippy --fix --allow-staged --allow-dirty --all-features
  just format

check-all:
  cargo check --all-features

check:
  cargo check

refresh:
  cargo clean && cargo update

watch:
  cargo watch -x lcheck

watch-test FEATURE='':
  #!/usr/bin/env sh
  if [ -z "{{FEATURE}}" ]; then
    cargo watch -x "nextest run"
  else
    cargo watch -x "nextest run --features {{FEATURE}}"
  fi

optimize CHAIN:
  scripts/build_release.sh -c {{CHAIN}}
