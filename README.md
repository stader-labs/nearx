# NearX - Liquid staking with crazy yields on NEAR

NearX is a liquid staking protocol offered by [Staderlabs](https://staderlabs.com "Staderlabs") on NEAR protocol. NearX unlocks your stake and allows you to participate in various DeFi protocols like Ref, Jumbo, Pembrok etc to earn yield on top of your staking rewards!

### Contracts in the repo

The NearX contracts are built with the near-rust-sdk(https://github.com/near/near-sdk-rs) 

1. nearx: The NearX contract which contains code related to NearX validator staking/unstaking, validator management and the NearX token.
2. mock-stake-pool: A mock validator stake pool contract which is used in the integration tests
3. integration-tests: A set of integration tests to test the NearX contract functionalities

### Building the project

We can build the NearX contract and the mock stake pool contract by running the following command

`make all
`

it moves all wasm files to res/ directory

To run the unit and integration tests, run the following

`make run-all-tests
`

### Integrating with NearX

A typescript sdk is available to integrate with NearX. Please refer to https://github.com/stader-labs/nearx-sdk

A nodejs based cli is available to at https://www.npmjs.com/package/nearx-cli

### Live contracts

The following are the contracts deployed on mainnet and testnet:

Mainnet NearX contract: `v2-nearx.stader-labs.near`

Testnet NearX contract: `v2-nearx.staderlabs.testnet`

Mainnet NearX Aurora Erc20: `0xb39eeb9e168ef6c639f5e282fef1f6bc4dcae375`

Testnet NearX Aurora Erc20: `0xfd2557bdf5bee20681690f21ceda22fd8135cda8`

Mainnet NearX Aurora Staking contract: `0x8E30eE730d4a6F3457befA60b25533F1400d31A6`

Testnet NearX Aurora Staking contract: `0xA2133C04Fed4301eD97e53067F51C238aBf9C810`

Mainnet NearX Near Price Feed contract: `nearx-price-feed.stader-labs.near`

Testnet NearX Near Price Feed contract: `nearx-price-feed.staderlabs.testnet`

Mainnet NearX Aurora Price Feed contract: `0x6081918387c97F81247adAF2F12a8A94A8dA84ED`

Testnet NearX Aurora Price Feed contract: `0x8CaD8F11aC35216071d796d1e6778C3C4d741bfA`

### Dapp link

The following is the link to the dapp

Dapp link: [near.staderlabs.com](https://near.staderlabs.com "near.staderlabs.com")

### Bug Bounty!

Stader Near has a bug bounty on NEAR. Please refer to the link here: https://immunefi.com/bounty/staderfornear/
