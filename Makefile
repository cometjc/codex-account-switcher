.PHONY: install build test

install:
	@n=$$(cat rust/plot-viewer/build-number 2>/dev/null || echo 0); echo $$(( n + 1 )) > rust/plot-viewer/build-number
	cargo install --locked --path rust/plot-viewer

build:
	cargo build

test:
	cargo test
