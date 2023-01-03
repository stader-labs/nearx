near view $CONTRACT_NAME get_nearx_pool_state

near view $CONTRACT_NAME get_nearx_price

near view $CONTRACT_NAME get_total_staked_balance

near view $CONTRACT_NAME get_validators

near view $CONTRACT_NAME get_total_validator_weight

near view $CONTRACT_NAME get_validator_info '{"validator": "'"$STAKE_POOL_0"'"}'

near view $CONTRACT_NAME get_account '{"account_id": "'"$ID"'"}'
near view $CONTRACT_NAME get_user_account '{"account_id": "'"$ID"'"}'

near view $CONTRACT_NAME get_number_of_accounts

near view $CONTRACT_NAME storage_balance_of '{"account_id": "'"$ID"'"}'

near view $CONTRACT_NAME get_accounts '{"from_index": 0, "limit": 40}'

near view $CONTRACT_NAME get_current_epoch

near view $CONTRACT_NAME get_reward_fee_fraction

near view $CONTRACT_NAME get_roles

near view $CONTRACT_NAME get_operations_control

# Storage deposit
near view $CONTRACT_NAME storage_balance_of '{"account_id": "'"$ID"'"}'

# FT queries
near view $CONTRACT_NAME ft_balance_of '{"account_id": "'"$ID"'"}'

near view $CONTRACT_NAME ft_total_supply

# Direct stake pool querying
near view $STAKE_POOL_0 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'

near view $STAKE_POOL_0 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_13 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'

near view $STAKE_POOL_0 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_2 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'

near view $CONTRACT_NAME storage_balance_of '{"account_id": "'"$ID"'"}'