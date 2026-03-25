.PHONY: install build test

install:
	cargo install --locked --path rust/plot-viewer

build:
	cargo build

test:
	cargo test
