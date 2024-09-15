build_macos:
	cargo dev_ui

build_server:
	cargo dev_server

build_all_features:
	cargo build --all-features

check_before_merge:build_all_features build_macos build_server
	echo "build all finished."