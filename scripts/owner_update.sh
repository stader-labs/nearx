
near call $CONTRACT_NAME set_owner '{"new_owner": "'"$ID"'"}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME commit_owner --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME set_reward_fee '{"numerator": 12, "denominator": 100}' --accountId=$ID --gas=300000000000000 --depositYocto=1;