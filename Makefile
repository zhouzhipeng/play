build_macos:
	cargo dev_ui

build_server:
	cargo dev_server

build_all_features:
	cargo build --all-features

update_cargo_dep:
	cargo  update

check_before_merge:
	# disable local `config.toml` temporarily
	mv ~/.cargo/config.toml ~/.cargo/config.toml.bak
	@$(shell $(MAKE) build_all_features) || (mv ~/.cargo/config.toml.bak ~/.cargo/config.toml; exit 1;)
	@$(shell $(MAKE) build_macos)
	@$(shell $(MAKE) build_server)
	# resume local config.toml
	mv ~/.cargo/config.toml.bak ~/.cargo/config.toml
	echo "build all finished."


build_wasm_example:
	./scripts/build_wasm_example.sh

build_dylib_example:
	cargo build --release -p play-dylib-example

build_linux_so:
	./scripts/build_linux_so.sh

