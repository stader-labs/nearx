# add some validators
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_0"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_1"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_2"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;

# update validator weight
near call $CONTRACT_NAME update_validator '{"validator": "'"$STAKE_POOL_6"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;

# pause validator
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_0"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_1"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_2"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_3"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_4"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_5"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_6"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_7"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_8"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_9"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_10"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_11"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_12"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_13"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_14"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_15"'"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;

