SHELL = /bin/bash

.ONESHELL:

.PHONY: setup build run generate train test-position

RUSTFLAGS = -C target-cpu=native
CARGO_ENV = source $$HOME/.cargo/env

define cargo-build
  $(CARGO_ENV); \
  RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin $(1)
endef

define cargo-run
  $(CARGO_ENV); \
  RUSTFLAGS="$(RUSTFLAGS)" cargo run --release --bin $(1)
endef

setup:
	sudo apt-get update
	sudo apt-get install -y curl build-essential
	# Install Rust non-interactively
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
	# Append Rust to future shell sessions
	echo 'source $$HOME/.cargo/env' >> $$HOME/.bashrc
	# Source it right now for *this* shell
	$(CARGO_ENV)

build-grail:
	$(call cargo-build,grail)
run-grail:
	$(call cargo-run,grail)

build-generate:
	$(call cargo-build,generate)
run-generate:
	$(call cargo-run,generate)

build-train:
	$(call cargo-build,train)
run-train:
	$(call cargo-run,train)

build-all: build-grail build-generate build-train

try: build
	$(CARGO_ENV)
	echo -e "position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1\ngo movetime 10000\nquit\n" | ./target/release/grail minimax

