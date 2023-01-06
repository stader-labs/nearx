# create lockup
near call lockup.near create '{"owner_account_id": "0e645c11a3d51b55ff4670f0baad82be417232ca1c16ce781f417d8bd8e35e3e", "lockup_duration": "86400"}' --accountId=$ID --gas=300000000000000 --amount=10;

# refresh stake pool balance
near call $CONTRACT_NAME refresh_staking_pool_balance --accountId=$ID --gas=300000000000000

# get total amount in staking pool
near call $CONTRACT_NAME get_known_deposited_balance

# get staking pool account id
near view $CONTRACT_NAME get_staking_pool_account_id

# get owner account id
near view $CONTRACT_NAME get_owner_account_id

# select staking pool
near call $CONTRACT_NAME select_staking_pool '{"staking_pool_account_id": "linear-protocol.near"}' --accountId=$ID --gas=300000000000000

# unselect staking pool
near call $CONTRACT_NAME unselect_staking_pool --accountId=$ID --gas=300000000000000

# deposit and stake
near call $CONTRACT_NAME deposit_and_stake '{"amount": "10000"}' --accountId=$ID --gas=300000000000000

# unstake
near call $CONTRACT_NAME unstake '{"amount": "10000"}' --accountId=$ID --gas=300000000000000

# withdraw
near call $CONTRACT_NAME withdraw_all_from_staking_pool --accountId=$ID --gas=300000000000000