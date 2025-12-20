.PHONY: build release deploy run

build:
	cargo build --release

run:
	cargo run --release

# Show what the next version would be based on commits
next-version:
	@./scripts/semver.sh --dry-run

# Create a new release tag based on conventional commits
deploy:
	@./scripts/semver.sh
