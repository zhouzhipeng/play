build_macos:
	cd  crates/play-ui && ./build-dmg.sh

build_server:
	cargo dev_server

build_all_features:
	cargo build --all-features

update_cargo_dep:
	cargo  update

check_before_merge:
	./scripts/check_before_merge.sh


build_wasm_example:
	./scripts/build_wasm_example.sh

build_dylib_example:
	cargo build --release -p play-dylib-example

build_linux_so:
	./scripts/build_linux_so.sh

