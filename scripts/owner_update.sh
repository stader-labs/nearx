
near call $CONTRACT_NAME set_owner '{"new_owner": "'"$ID"'"}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME commit_owner --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME set_reward_fee '{"numerator": 12, "denominator": 100}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME update_operations_control '{"update_operations_control_request": {
  "ft_transfer_paused": false,
  "ft_transfer_call_paused": false
}}' --accountId=$ID --gas=300000000000000 --depositYocto=1;