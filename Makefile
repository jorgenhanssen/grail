SHELL = /bin/bash

.ONESHELL:

.PHONY: grail grail-tuning generate train clean

# Default to native optimization for local development.
RUSTFLAGS = -C target-cpu=native

grail:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin grail

grail-tuning:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin grail --features tuning

generate:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin generate

train:
	@GPU_FEATURES=$$([ "$$(uname -s)" = "Darwin" ] && echo metal || (command -v nvcc >/dev/null 2>&1 && echo cuda || true)); \
	if [ -n "$$GPU_FEATURES" ]; then \
		RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p nnue --bin train --features $$GPU_FEATURES; \
	else \
		RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p nnue --bin train; \
	fi

clean:
	cargo clean
