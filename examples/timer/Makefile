build:
	@RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
	@cp target/wasm32-unknown-unknown/release/example.wasm .
lint:
	@cargo fmt
serve:
	@python3 -m http.server