all: build

build:
	cargo build

fmt:
	cargo fmt

check: fmt
	cargo clippy

