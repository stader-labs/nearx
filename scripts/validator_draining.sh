near call $CONTRACT_NAME drain_unstake '{"validator": "'"$STAKE_POOL_1"'"}' --accountId=$ID --gas=300000000000000;

near call $CONTRACT_NAME drain_withdraw '{"validator": "'"$STAKE_POOL_14"'"}' --accountId=$ID --gas=300000000000000;