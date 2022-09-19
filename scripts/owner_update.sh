near call $CONTRACT_NAME set_owner '{"new_owner": "'"$ID"'"}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME set_operator_id '{"new_operator_account_id": "'"$ID"'"}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME set_treasury_id '{"new_treasury_account_id": "'"$ID"'"}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME commit_owner --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME commit_operator_id --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME commit_treasury_id --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME set_reward_fee '{"numerator": 10, "denominator": 100}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME commit_reward_fee --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME add_min_storage_reserve --accountId=$ID --gas=300000000000000 --amount=10;

near call $CONTRACT_NAME update_rewards_buffer --accountId=$ID --gas=300000000000000 --depositYocto=1596381189754760206691;

near call $CONTRACT_NAME transfer_funds '{"account_id": "'"$ID"'", "amount": "1000000000000000000000000"}' --accountId=$ID --gas=300000000000000 --amount=10;