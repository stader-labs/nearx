near dev-deploy --wasmFile res/nearX.wasm

# Deploy and init should be a batch call
near call $CONTRACT_NAME new '{"owner_account_id": "'"$ID"'", "operator_account_id": "'"$ID"'"}' --accountId=$ID

STAKE_POOL_0=legends.pool.f863973.m0
STAKE_POOL_1=masternode24.pool.f863973.m0
STAKE_POOL_2=01node.pool.f863973.m0

# add some validators
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_0"'"}' --accountId=$ID
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_1"'"}' --accountId=$ID
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_2"'"}' --accountId=$ID

# manager deposit

for i in {1..3};
do near call $CONTRACT_NAME manager_deposit_and_stake --accountId=$ID --amount=3 --gas=300000000000000;
done;

# 10 deposits
for i in {1..10};
do near call $CONTRACT_NAME deposit_and_stake --accountId=$ID --amount=1 --gas=300000000000000;
done;

near call $CONTRACT_NAME unstake '{"amount": "3000000000000000000000000"}' --accountId=$ID --gas=300000000000000;

# epoch stake
near call $CONTRACT_NAME epoch_stake --accountId=$ID --gas=300000000000000;

# get contract state
near view $CONTRACT_NAME get_nearx_pool_state

near view $CONTRACT_NAME get_nearx_price

near view $CONTRACT_NAME get_total_staked

near view $CONTRACT_NAME get_validators

# get staking pool
near view $CONTRACT_NAME get_validator_info '{"validator": "'"$STAKE_POOL_0"'"}'
near view $CONTRACT_NAME get_validator_info '{"validator": "'"$STAKE_POOL_1"'"}'
near view $CONTRACT_NAME get_validator_info '{"validator": "'"$STAKE_POOL_2"'"}'

# get user state
near view $CONTRACT_NAME get_account '{"account_id":  "'"$ID"'"}'

# Reward distribution
near call $CONTRACT_NAME epoch_autocompound_rewards '{"validator": "'"$STAKE_POOL_0"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME epoch_autocompound_rewards '{"validator": "'"$STAKE_POOL_1"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME epoch_autocompound_rewards '{"validator": "'"$STAKE_POOL_2"'"}' --accountId=$ID --gas=300000000000000

near view $CONTRACT_NAME ft_balance_of '{"account_id": "'"$ID"'"}'
near view $CONTRACT_NAME ft_total_supply

# Checking stake in the stake pool contract
near view $STAKE_POOL_0 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'

near view $STAKE_POOL_0 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_2 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'

near view $STAKE_POOL_0 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_2 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
