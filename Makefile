build_macos:
	cargo dev_ui

build_server:
	cargo dev_server

build_all_features:
	cargo build --all-features

build_all:build_macos build_server build_all_features
	echo "build all finished."