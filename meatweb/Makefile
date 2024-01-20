.PHONY: build serve release

RUSTFLAGS := --cfg=web_sys_unstable_apis

default:
	@echo "No target specified. Run 'make build', 'make serve', or 'make release'."

build:
	RUSTFLAGS=$(RUSTFLAGS) trunk build

serve:
	RUSTFLAGS=$(RUSTFLAGS) trunk serve

release:
	RUSTFLAGS=$(RUSTFLAGS) trunk build --release
	wasm-opt -Oz -o dist/*_bg.wasm  dist/*_bg.wasm
