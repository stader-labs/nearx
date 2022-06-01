RFLAGS="-C link-arg=-s"

all: linear

linear:
	#rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo +stable build --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/*.wasm ./res

#RUSTFLAGS='-C link-arg=-s' cargo +stable build --all --target wasm32-unknown-unknown --release
#cp -u target/wasm32-unknown-unknown/release/metapool.wasm res/
