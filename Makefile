RFLAGS="-C link-arg=-s"

all: near-x mock-stake-pool

integration-tests: all run-integration-tests

unit-tests: run-unit-tests

run-all-tests: all run-unit-tests run-integration-tests

near-x: contracts/near-x
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo +stable build -p near-x --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/near_x.wasm ./res/near_x.wasm

promote-nearx-to-migratable:
	cp res/near_x.wasm live_wasms/near_x.wasm

mock-stake-pool: contracts/mock-stake-pool
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo build -p mock-stake-pool --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/mock_stake_pool.wasm ./res/mock_stake_pool.wasm

run-integration-tests: contracts/integration-tests
	RUSTFLAGS=$(RFLAGS) cargo test

run-unit-tests: contracts/near-x
	RUSTFLAGS=$(RFLAGS) cargo test