build FEATURES='':
  #!/usr/bin/env sh
  echo "-- Building {{FEATURES}} -- \n"
  if [ -z "{{FEATURES}}" ]; then
    cargo build
  else
    cargo build --features {{FEATURES}}
  fi

test FEATURES='':
  #!/usr/bin/env sh
  if [ -z "{{FEATURES}}" ]; then
    cargo test
  else
    cargo test --features {{FEATURES}}
  fi

format:
  cargo fmt --all
  find . -type f -iname "*.toml" -print0 | xargs -0 taplo format

lint:
  cargo clippy --all --all-features -- -D warnings

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

watch-test:
  cargo watch -x "nextest run"

optimize CHAIN:
  ./build_release.sh --c {{CHAIN}}
