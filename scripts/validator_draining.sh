near call $CONTRACT_NAME drain_unstake '{"validator": "'"$STAKE_POOL_0"'"}' --accountId=$ID --gas=300000000000000;

near call $CONTRACT_NAME drain_withdraw '{"validator": "'"$STAKE_POOL_0"'"}' --accountId=$ID --gas=300000000000000;