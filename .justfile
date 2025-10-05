lint:
  cargo fmt --all
  cargo clippy --fix

fmt:
  cargo fmt --check

clippy:
  cargo clippy
