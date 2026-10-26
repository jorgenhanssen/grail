SHELL = /bin/bash

.ONESHELL:

.PHONY: grail grail-pgo datagen datagen-pgo train dataset-analysis clean profile tuner tuner-pgo

# Default to native optimization for local development.
RUSTFLAGS = -C target-cpu=native

grail:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --bin grail

grail-pgo:
	RUSTFLAGS="$(RUSTFLAGS)" bash scripts/pgo.sh "--bin grail" "./target/release/grail bench"

datagen:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p training --bin datagen

datagen-pgo:
	RUSTFLAGS="$(RUSTFLAGS)" bash scripts/pgo.sh "-p training --bin datagen" \
		"./target/release/datagen --random-plies 8 --nodes 10000 --threads 1 --max-games 100 --dry-run"

test:
	RUSTFLAGS="$(RUSTFLAGS)" cargo test

train:
	@if command -v nvcc >/dev/null 2>&1; then \
		RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p training --bin train --features cuda; \
	else \
		RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p training --bin train; \
	fi

dataset-analysis:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p training --bin dataset_analysis

tuner:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release -p tuning --bin tuner

tuner-pgo:
	RUSTFLAGS="$(RUSTFLAGS)" bash scripts/pgo.sh "-p tuning --bin tuner" \
		"./target/release/tuner --params tuning/params.example.toml --book books/UHO_Lichess_4852_v1.epd --pairs 5 --iterations 3 --workers 1"

profile:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --profile profiling -p grail --bin grail
	samply record -o profile.json.gz -- ./target/profiling/grail bench

clean:
	cargo clean
