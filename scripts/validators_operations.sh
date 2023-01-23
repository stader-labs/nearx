# add some validators
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_0"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_1"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_2"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_3"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_4"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_5"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_6"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_7"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_8"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_9"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_10"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_11"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_12"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;
near call $CONTRACT_NAME add_validator '{"validator": "'"$STAKE_POOL_13"'", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;

# update validator weight
near call $CONTRACT_NAME update_validator '{"validator": "inc4.poolv1.near", "weight": 10}' --accountId=$ID --depositYocto=1 --gas=300000000000000;

# pause validator
near call $CONTRACT_NAME pause_validator '{"validator": "pathrocknetwork.poolv1.near"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME make_validator_private '{"validator": "stakin.poolv1.near", "initial_max_unstakable_limit": "58794775715936503923304176597" }' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME make_validator_public '{"validator": "rekt.poolv1.near" }' --accountId=$ID --gas=300000000000000 --depositYocto=1;

# update max unstakable limit
near call $CONTRACT_NAME update_validator_max_unstakeable_limit '{"validator": "epic.poolv1.near", "amount_unstaked": "2000000000000000000000000"}' --accountId=$ID  --gas=300000000000000 --depositYocto=1;