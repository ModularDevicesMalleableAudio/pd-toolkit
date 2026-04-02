PROJECT_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
TOOLS_DIR   := $(PROJECT_DIR)../.tools
PI_TARGET   := aarch64-unknown-linux-musl
PI_HOST     := instruments@pisound.local

.PHONY: build release test cross-pi install-local install deploy lint clean


# Fast dev build with debug symbols
build:
	cargo build

# Optimised release build (strips debug info, enables LTO)
release:
	cargo build --release

# Run all tests (unit + integration + golden)
test:
	cargo test

# Lint Rust code with Clippy and check formatting
lint:
	cargo clippy -- -W clippy::all
	cargo fmt --check

# Remove all build artefacts, generated man pages and completions
clean:
	cargo clean
	rm -rf man/ completions/

# Cross-compilation

# Cross-compile a fully static musl binary for Raspberry Pi 4 (aarch64)
# Requires: cargo install cross   (uses Docker)
cross-pi:
	cross build --release --target $(PI_TARGET)

# Installation

# Install into the parent project's .tools directory
install-local: release
	mkdir -p $(TOOLS_DIR)/bin $(TOOLS_DIR)/man/man1
	cp target/release/pdtk $(TOOLS_DIR)/bin/
	cp man/*.1 $(TOOLS_DIR)/man/man1/ 2>/dev/null || true
	@echo "Installed to $(TOOLS_DIR)"

# Install system-wide (requires sudo)
install: release
	sudo install -m 755 target/release/pdtk /usr/local/bin/
	sudo mkdir -p /usr/local/share/man/man1
	sudo install -m 644 man/*.1 /usr/local/share/man/man1/ 2>/dev/null || true
	sudo mandb 2>/dev/null || true
	@echo "Installed system-wide"

# Deployment

# Cross-compile and deploy the static binary to the Raspberry Pi over SSH
deploy: cross-pi
	scp target/$(PI_TARGET)/release/pdtk \
	    $(PI_HOST):/home/instruments/code/sequencer/.tools/bin/
	@if ls man/*.1 >/dev/null 2>&1; then \
	    scp man/*.1 \
	        $(PI_HOST):/home/instruments/code/sequencer/.tools/man/man1/; \
	fi
	@echo "Deployed to $(PI_HOST)"
