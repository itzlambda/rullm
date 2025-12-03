lint:
  cargo fmt --all
  cargo clippy --all-targets --all-features -- -D warnings


fmt:
  cargo fmt --check

clippy:
  cargo clippy
