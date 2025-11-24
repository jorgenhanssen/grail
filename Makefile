SHELL = /bin/bash

.ONESHELL:

.PHONY: grail grail-tuning generate train clean

grail:
	cargo build --release --bin grail

grail-tuning:
	cargo build --release --bin grail --features tuning

generate:
	cargo build --release --bin generate

train:
	@GPU_FEATURES=$$([ "$$(uname -s)" = "Darwin" ] && echo metal || (command -v nvcc >/dev/null 2>&1 && echo cuda || true)); \
	if [ -n "$$GPU_FEATURES" ]; then \
		cargo build --release -p nnue --bin train --features $$GPU_FEATURES; \
	else \
		cargo build --release -p nnue --bin train; \
	fi

clean:
	cargo clean
