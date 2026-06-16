SHELL = /bin/bash

.ONESHELL:

.PHONY: grail tunable generate train clean nnue-analysis flamegraph

# Default to native optimization for local development.
RUSTFLAGS = -C target-cpu=native

grail:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin grail

tunable:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin grail --features tuning

generate:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p training --bin generate

test:
	RUSTFLAGS="$(RUSTFLAGS)" cargo test

train:
	@GPU_FEATURES=$$([ "$$(uname -s)" = "Darwin" ] && echo metal || (command -v nvcc >/dev/null 2>&1 && echo cuda || true)); \
	if [ -n "$$GPU_FEATURES" ]; then \
		RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p training --bin train --features $$GPU_FEATURES; \
	else \
		RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p training --bin train; \
	fi

nnue-analysis:
	RUSTFLAGS="$(RUSTFLAGS)" cargo run --release -p nnue --bin nnue-analysis

flamegraph:
	@ROOT=$$([ "$$(uname -s)" = "Linux" ] && echo --root); \
	RUSTFLAGS="$(RUSTFLAGS)" cargo flamegraph --profile profiling -p grail --bin grail $$ROOT -o flamegraph.svg -- bench

clean:
	cargo clean
