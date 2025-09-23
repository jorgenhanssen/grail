SHELL = /bin/bash

.ONESHELL:

.PHONY: setup build run build-grail run-grail build-generate run-generate build-train run-train build-train-cuda run-train-cuda build-tournament build-all

RUSTFLAGS = -C target-cpu=native

run: build-grail
	echo -e "position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1\ngo movetime 10000\nquit\n" | ./target/release/grail

setup:
	sudo apt-get update
	sudo apt-get install -y curl build-essential
	# Install Rust non-interactively
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
	# Append Rust to future shell sessions
	echo 'source $$HOME/.cargo/env' >> $$HOME/.bashrc
	# Source it right now for *this* shell (optional if already in shell rc)

build-grail:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin grail
run-grail:
	RUSTFLAGS="$(RUSTFLAGS)" cargo run --release --bin grail

build-grail-tuning:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin grail --features tuning
run-grail-tuning:
	RUSTFLAGS="$(RUSTFLAGS)" cargo run --release --bin grail --features tuning
build-grail-nnue:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin grail --features nnue
run-grail-nnue:
	RUSTFLAGS="$(RUSTFLAGS)" cargo run --release --bin grail --features nnue

build-generate:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin generate
run-generate:
	RUSTFLAGS="$(RUSTFLAGS)" cargo run --release --bin generate

build-train:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin train
run-train:
	RUSTFLAGS="$(RUSTFLAGS)" cargo run --release --bin train

build-train-cuda:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p nnue --bin train --features cuda
run-train-cuda:
	RUSTFLAGS="$(RUSTFLAGS)" cargo run --release -p nnue --bin train --features cuda

build-tournament:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin tournament

build-all: build-grail build-generate build-train build-tournament