SHELL = /bin/bash

.ONESHELL:

.PHONY: setup build run build-grail build-grail-tuning build-generate build-train build-train-cuda build-train-metal build-train-cpu build-all

RUSTFLAGS = -C target-cpu=native

# "cuda" or "metal" or empty string
GPU_FEATURES := $(shell [ "$$(uname -s)" = "Darwin" ] && echo metal || (command -v nvcc >/dev/null 2>&1 && echo cuda || true))

grail:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin grail

grail-tuning:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin grail --features tuning
	
generate:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin generate

train:
	@echo "Building with features: $(GPU_FEATURES)"
	@if [ -n "$(GPU_FEATURES)" ]; then \
		RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p nnue --bin train --features $(GPU_FEATURES); \
	else \
		RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p nnue --bin train; \
	fi

setup:
	sudo apt-get update
	sudo apt-get install -y curl build-essential
	# Install Rust non-interactively
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
	# Append Rust to future shell sessions
	echo 'source $$HOME/.cargo/env' >> $$HOME/.bashrc
	# Source it right now for *this* shell (optional if already in shell rc)
