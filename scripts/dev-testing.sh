# add some validators
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_0"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_1"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_2"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;

# manager deposit
near call $CONTRACT_NAME manager_deposit_and_stake '{"validator": "'"$STAKE_POOL_0"'"}'  --accountId=$ID --amount=1 --gas=300000000000000;
near call $CONTRACT_NAME manager_deposit_and_stake '{"validator": "'"$STAKE_POOL_1"'"}'  --accountId=$ID --amount=1 --gas=300000000000000;
near call $CONTRACT_NAME manager_deposit_and_stake '{"validator": "'"$STAKE_POOL_12"'"}'  --accountId=$ID --amount=0.1 --gas=300000000000000;

# 10 deposits
for i in {1..3};
do
  near call $CONTRACT_NAME storage_deposit --accountId=$ID --amount=0.1 --gas=300000000000000;
  near call $CONTRACT_NAME deposit_and_stake --accountId=$ID --amount=5 --gas=300000000000000;
done;

near call $CONTRACT_NAME storage_unregister --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME unstake '{"amount": "1000000000000000000000000"}' --accountId=$ID --gas=300000000000000;

near call $CONTRACT_NAME unstake_all --accountId=$ID --gas=300000000000000;

near call $CONTRACT_NAME withdraw_all --accountId=$ID --gas=300000000000000;

near call $CONTRACT_NAME withdraw '{"amount": "3997500000000000000000000"}' --accountId=$ID --gas=300000000000000;

near call $CONTRACT_NAME storage_deposit --accountId=$ID --gas=300000000000000 --depositYocto=3000000000000000000000;

near view $CONTRACT_NAME ft_balance_of '{"account_id": "'"$ID"'"}'
near view $CONTRACT_NAME ft_total_supply

# Checking stake in the stake pool contract
near view $STAKE_POOL_0 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'

near view $STAKE_POOL_10 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_2 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'

near view $STAKE_POOL_0 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_2 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
