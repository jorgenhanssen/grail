SHELL = /bin/bash

.ONESHELL:

.PHONY: grail tunable generate train clean nnue-analysis profile

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

profile:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --profile profiling -p grail --bin grail
	samply record -o profile.json.gz -- ./target/profiling/grail bench

clean:
	cargo clean
