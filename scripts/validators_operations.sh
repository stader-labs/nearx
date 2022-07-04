STAKE_POOL_0=everstake.pool.f863973.m0
STAKE_POOL_1=infstones.pool.f863973.m0
STAKE_POOL_2=ni.pool.f863973.m0

# add some validators
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_0"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_1"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_2"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;

# update validator weight
near call $CONTRACT_NAME update_validator '{"validator": "'"$STAKE_POOL_0"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;

# pause validator
near call $CONTRACT_NAME pause_validator '{"validator": "'"$STAKE_POOL_0"'"}' --accountId=$ID  --gas=300000000000000;