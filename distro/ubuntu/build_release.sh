# sudo apt update
# sudo apt install musl-tools
# cargo clean
# cargo update

cargo build --release --target x86_64-unknown-linux-gnu
cp -f target/x86_64-unknown-linux-gnu/release/stackmuncher stackmuncher-x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-musl
cp -f target/x86_64-unknown-linux-musl/release/stackmuncher stackmuncher-x86_64-unknown-linux-musl
