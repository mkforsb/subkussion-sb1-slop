# Convenience targets. Requires the Dioxus CLI (`dx`, 0.7.x) and the
# wasm32-unknown-unknown target (`rustup target add wasm32-unknown-unknown`).

APP := crates/app

.PHONY: web desktop serve-web run-desktop test lint renders bench calibrate clean

## Serve the web build with hot reload on http://127.0.0.1:8080
serve-web:
	cd $(APP) && dx serve --web --release

## Run the native Linux app (PulseAudio / PipeWire-pulse)
run-desktop:
	cd $(APP) && dx serve --desktop --release

## Release bundles: target/dx/subkussion/release/{web/public,linux/app}
web:
	cd $(APP) && dx build --web --release

desktop:
	cd $(APP) && dx build --desktop --release

test:
	cargo test --release -p subkussion-dsp -p subkussion

lint:
	cargo fmt --all --check
	cargo clippy --all-targets -p subkussion-dsp -p subkussion-worklet -- -D warnings
	cargo clippy --all-targets -p subkussion --features desktop -- -D warnings
	cargo clippy -p subkussion --features web --target wasm32-unknown-unknown -- -D warnings

## Render every factory preset to WAV files in ./renders
renders:
	cargo run --release -p subkussion-dsp --example render -- renders

bench:
	cargo run --release -p subkussion-dsp --example bench

## Loudness survey of every engine across pitch/click/macro settings
calibrate:
	cargo run --release -p subkussion-dsp --example calibrate

clean:
	cargo clean
