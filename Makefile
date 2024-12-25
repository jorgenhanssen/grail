ARGS=

build:
	./scripts/build.sh

run: build
	./scripts/run.sh $(ARGS)

test: build
	./scripts/test.sh $(ARGS)