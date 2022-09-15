
near call $CONTRACT_NAME set_owner '{"new_owner": "'"$ID"'"}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME set_operator_id '{"new_operator_account_id": "'"$ID"'"}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME set_treasury_id '{"new_treasury_account_id": "'"$ID"'"}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME commit_owner --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME commit_operator_id --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME commit_treasury_id --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME set_reward_fee '{"numerator": 12, "denominator": 100}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME commit_reward_fee --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME update_operations_control '{"update_operations_control_request": {
  "ft_transfer_paused": true,
  "ft_transfer_call_paused": true,
  "stake_paused": false,
  "unstake_paused": true,
  "withdraw_paused": true,
  "staking_epoch_paused": true,
  "unstaking_epoch_paused": true,
  "withdraw_epoch_paused": true,
  "autocompounding_epoch_paused": false,
  "sync_validator_balance_paused": true
}}' --accountId=$ID --gas=300000000000000 --depositYocto=1;

near call $CONTRACT_NAME add_min_storage_reserve --accountId=$ID --gas=300000000000000 --amount=10;