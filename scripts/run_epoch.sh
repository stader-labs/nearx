# An epoch involves autocompounding, syncing validator balances, stake epoch, unstake epoch

near call $CONTRACT_NAME epoch_autocompound_rewards '{"validator": "'"$STAKE_POOL_0"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME epoch_autocompound_rewards '{"validator": "'"$STAKE_POOL_1"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME epoch_autocompound_rewards '{"validator": "'"$STAKE_POOL_2"'"}' --accountId=$ID --gas=300000000000000

near call $CONTRACT_NAME epoch_stake --accountId=$ID --gas=300000000000000;

# run till false is returned
near call $CONTRACT_NAME epoch_unstake --accountId=$ID --gas=300000000000000;

near call $CONTRACT_NAME epoch_withdraw '{"validator": "'"$STAKE_POOL_0"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME epoch_withdraw '{"validator": "'"$STAKE_POOL_1"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME epoch_withdraw '{"validator": "'"$STAKE_POOL_2"'"}' --accountId=$ID --gas=300000000000000

near call $CONTRACT_NAME sync_balance_from_validator '{"validator_id": "'"$STAKE_POOL_0"'"}' --accountId=$ID --gas=300000000000000;
near call $CONTRACT_NAME sync_balance_from_validator '{"validator_id": "'"$STAKE_POOL_1"'"}' --accountId=$ID --gas=300000000000000;
near call $CONTRACT_NAME sync_balance_from_validator '{"validator_id": "'"$STAKE_POOL_2"'"}' --accountId=$ID --gas=300000000000000;

