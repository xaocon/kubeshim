set dotenv-load := true

list:
	@just --list

build:
	cargo build --release

install: build
	strip target/release/kubeshim
	cargo install --path .

run:
	cargo run

upgrade:
	cargo upgrade
	cargo update

bump level="patch":
	cargo set-version --bump {{ level }}

clean:
	cargo clean

test:
	markdownlint -f *.md
	cargo check
	cargo test
	cargo fmt
	cargo clippy
