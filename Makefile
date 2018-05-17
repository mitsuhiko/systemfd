all: test

doc:
	@cargo doc

test: cargotest

cargotest:
	@cargo test

format-check:
	@cargo fmt -- --write-mode diff

.PHONY: all doc test cargotest format-check
