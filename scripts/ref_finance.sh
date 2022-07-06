
# add pool
near call $CONTRACT_ID add_simple_pool "{\"tokens\": [\"$TOKEN1\", \"$TOKEN2\"], \"fee\": 25}" --accountId $USER_ID --amount 0.1

# register for storage
near call $CONTRACT_ID storage_deposit '' --accountId $USER_ID --amount 1

# register tokens
near call $CONTRACT_ID register_tokens '{"token_ids": ["'"$TOKEN1"'", "'"$TOKEN2"'"]}' --accountId $USER_ID --depositYocto 1

# token transfer

# our token
near call $TOKEN1 ft_transfer_call '{"receiver_id": "'"$CONTRACT_ID"'", "amount": "2000000000000000000000000", "msg": ""}' --accountId $USER_ID --amount 0.000000000000000000000001 --gas=300000000000000;
# wrapped near
near call $WRAPPED_NEAR_CONTRACT ft_transfer_call '{"receiver_id": "'"$CONTRACT_ID"'", "amount": "2000000000000000000000000", "msg": ""}' --accountId $USER_ID --amount 0.000000000000000000000001 --gas=300000000000000;

# get wrapped near
near call $WRAPPED_NEAR_CONTRACT near_deposit --accountId $USER_ID --amount 5

# add liquidity
near call $CONTRACT_ID add_liquidity '{"pool_id": 181, "amounts": ["1", "1"]}' --accountId $USER_ID --amount 0.000000000000000000000001 --gas=300000000000000;

near view $CONTRACT_ID get_pool_shares '{"pool_id": 181, "account_id": "'"$USER_ID"'"}'