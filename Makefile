all: test

doc:
	@cargo doc

test: cargotest

cargotest:
	@cargo test

format-check:
	@cargo fmt -- --check

.PHONY: all doc test cargotest format-check
