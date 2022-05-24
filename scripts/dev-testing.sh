near dev-deploy --wasmFile res/nearX.wasm

# Deploy and init should be a batch call
near call $CONTRACT_NAME new '{"owner_account_id": "'"$ID"'", "operator_account_id": "'"$ID"'"}' --accountId=$ID

# add some validators
near call $CONTRACT_NAME add_staking_pool '{"account_id": "legends.pool.f863973.m0"}' --accountId=$ID
near call $CONTRACT_NAME add_staking_pool '{"account_id": "masternode24.pool.f863973.m0"}' --accountId=$ID

# 10 deposits
for i in {1..10};
do near call $CONTRACT_NAME deposit_and_stake --accountId=$ID --amount=1 --gas=300000000000000;
done;

# get contract state
near view $CONTRACT_NAME get_near_pool_state

near view $CONTRACT_NAME get_nearx_price

near view $CONTRACT_NAME get_total_staked

near view $CONTRACT_NAME get_stake_pools

# get staking pool
near view $CONTRACT_NAME get_sp_info '{"inx": 0}'
near view $CONTRACT_NAME get_sp_info '{"inx": 1}'
near view $CONTRACT_NAME get_sp_info '{"inx": 2}'

# get user state
near view $CONTRACT_NAME get_account '{"account_id":  "'"$ID"'"}'

# Reward distribution
near call $CONTRACT_NAME distribute_rewards '{"sp_inx": 0}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME distribute_rewards '{"sp_inx": 1}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME distribute_rewards '{"sp_inx": 2}' --accountId=$ID --gas=300000000000000

near view $CONTRACT_NAME ft_balance_of '{"account_id": "learning12345.testnet"}'
near view $CONTRACT_NAME ft_total_supply

STAKE_POOL_0=legends.pool.f863973.m0
STAKE_POOL_1=masternode24.pool.f863973.m0
STAKE_POOL_2=01node.pool.f863973.m0
STAKE_POOL_3=p2p.pool.f863973.m0
STAKE_POOL_4=nodeasy.pool.f863973.m0
STAKE_POOL_5=chorusone.pool.f863973.m0
STAKE_POOL_6=foundryusa.pool.f863973.m0
STAKE_POOL_7=lunanova2.pool.f863973.m0
STAKE_POOL_8=chorus-one.pool.f863973.m0
STAKE_POOL_9=ni.pool.f863973.m0
STAKE_POOL_10=cryptogarik.pool.f863973.m0
STAKE_POOL_11=stakely_v2.pool.f863973.m0

# Checking stake in the stake pool contract
near view $STAKE_POOL_0 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'

near view $STAKE_POOL_0 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_2 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'

near view $STAKE_POOL_0 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'