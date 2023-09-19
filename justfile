# You can install `just` from here > https://github.com/casey/just

default: clean check test build
all: clean check test bench build

clean:
    cargo clean

check:
    cargo fmt --check
    RUSTFLAGS='-D warnings' cargo check --tests
    cargo clippy -- -D warnings
    cargo audit --deny unsound --deny yanked --target-os linux
    cargo audit --deny unsound --deny yanked --target-os macos

test:
    cargo test
    cargo test --examples
    cargo tarpaulin --out html

bench:
    cargo bench

build:
    cargo clean
    cargo build --release