build:
	@RUSTFLAGS='-C link-arg=-s' cargo build --release
	@cp target/wasm32-unknown-unknown/release/example.wasm .
serve:
	@python3 -m http.server 8080
lint:
	@cargo fmt