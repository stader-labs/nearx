near call $CONTRACT_NAME ft_transfer '{"receiver_id": "'"$ID"'", "amount": "20659988951999999999999999"}' --accountId=$ID --depositYocto=1 --gas=300000000000000;

near call $CONTRACT_NAME ft_transfer_call '{"receiver_id": "'"$ID"'", "amount": "3000000000000000000000000", "msg": "" }' --depositYocto=1 --accountId=$ID --gas=300000000000000;
