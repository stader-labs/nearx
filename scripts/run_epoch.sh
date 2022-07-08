# An epoch involves autocompounding, syncing validator balances, stake epoch, unstake epoch
STAKE_POOL_0=stake1.poolv1.near
STAKE_POOL_1=optimusvalidatornetwork.poolv1.near
STAKE_POOL_2=appload.poolv1.near
STAKE_POOL_3=stakesstone.poolv1.near
STAKE_POOL_4=staking-power.poolv1.near
STAKE_POOL_5=chorusone.poolv1.near
STAKE_POOL_6=dragonfly.poolv1.near
STAKE_POOL_7=cryptium.poolv1.near
STAKE_POOL_8=magic.poolv1.near
STAKE_POOL_9=rekt.poolv1.near
STAKE_POOL_10=dokiacapital.poolv1.near
STAKE_POOL_11=jumbo_exchange.pool.near


near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_0"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_1"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_2"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_3"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_4"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_5"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_6"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_7"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_8"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_9"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_10"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME autocompounding_epoch '{"validator": "'"$STAKE_POOL_11"'"}' --accountId=$ID --gas=300000000000000

near call $CONTRACT_NAME staking_epoch --accountId=$ID --gas=300000000000000;

# run till false is returned
near call $CONTRACT_NAME unstaking_epoch --accountId=$ID --gas=300000000000000;

near call $CONTRACT_NAME withdraw_epoch '{"validator": "'"$STAKE_POOL_0"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME withdraw_epoch '{"validator": "'"$STAKE_POOL_1"'"}' --accountId=$ID --gas=300000000000000
near call $CONTRACT_NAME withdraw_epoch '{"validator": "'"$STAKE_POOL_2"'"}' --accountId=$ID --gas=300000000000000

near call $CONTRACT_NAME sync_balance_from_validator '{"validator_id": "'"$STAKE_POOL_0"'"}' --accountId=$ID --gas=300000000000000;
near call $CONTRACT_NAME sync_balance_from_validator '{"validator_id": "'"$STAKE_POOL_1"'"}' --accountId=$ID --gas=300000000000000;
near call $CONTRACT_NAME sync_balance_from_validator '{"validator_id": "'"$STAKE_POOL_2"'"}' --accountId=$ID --gas=300000000000000;

