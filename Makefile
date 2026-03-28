.PHONY: install build test exe

# Windows x86_64 GNU (.exe); cross-build from Linux/macOS with mingw + rustup target.
WINDOWS_TARGET := x86_64-pc-windows-gnu

install:
	@build_number=$$(cat build-number 2>/dev/null || \
		git rev-list --count HEAD 2>/dev/null || echo 0); \
	BUILD_NUMBER=$${build_number} cargo install --locked --path .

build:
	cargo build

test:
	cargo test

# Run tests, then release-build agent-switch + cursor-export for Windows.
# Requires: rustup, mingw linker (see .cargo/config.toml).
# Prints absolute paths to the resulting .exe files.
exe:
	@command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1 || { \
		echo >&2 "error: x86_64-w64-mingw32-gcc not found (needed for ring/rustls cross-build)."; \
		echo >&2 "  Debian/Ubuntu: sudo apt install gcc-mingw-w64-x86-64"; \
		exit 1; \
	}
	@set -e; \
	build_number=$$(cat build-number 2>/dev/null || \
		git rev-list --count HEAD 2>/dev/null || echo 0); \
	echo "cargo test --locked ..."; \
	BUILD_NUMBER=$$build_number cargo test --locked; \
	echo "rustup target add $(WINDOWS_TARGET) ..."; \
	rustup target add $(WINDOWS_TARGET); \
	echo "cargo build --locked --release --target $(WINDOWS_TARGET) ..."; \
	BUILD_NUMBER=$$build_number cargo build --locked --release \
		--target $(WINDOWS_TARGET) \
		--bin agent-switch --bin cursor-export; \
	echo ""; \
	echo "Windows release binaries:"; \
	echo "  $(CURDIR)/target/$(WINDOWS_TARGET)/release/agent-switch.exe"; \
	echo "  $(CURDIR)/target/$(WINDOWS_TARGET)/release/cursor-export.exe"
