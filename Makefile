.PHONY: install build test

install:
	@build_number=$$(cat rust/plot-viewer/build-number 2>/dev/null || \
		git -C rust/plot-viewer rev-list --count HEAD 2>/dev/null || echo 0); \
	BUILD_NUMBER=$${build_number} cargo install --locked --path rust/plot-viewer

build:
	cargo build

test:
	cargo test
