# An epoch involves autocompounding, syncing validator balances, stake epoch, unstake epoch

near call $CONTRACT_NAME epoch_autocompound_rewards '{"validator": "'"$STAKE_POOL_0"'"}' --accountId=$ID --gas=300000000000000

near call $CONTRACT_NAME epoch_stake --accountId=$ID --gas=300000000000000;

# run till false is returned
near call $CONTRACT_NAME epoch_unstake --accountId=$ID --gas=300000000000000;

near call $CONTRACT_NAME sync_validator_balances --accountId=$ID --gas=300000000000000;

