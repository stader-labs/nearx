RFLAGS="-C link-arg=-s"

all: near-x mock-stake-pool

near-x: contracts/near-x
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo +stable build -p near-x --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/near_x.wasm ./res/near_x.wasm

mock-stake-pool: contracts/mock-stake-pool
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo build -p mock-stake-pool --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/mock_stake_pool.wasm ./res/mock_stake_pool.wasm