
# deposits
for i in {1..10};
  do near call $CONTRACT_NAME deposit_and_stake --accountId=$ID --amount=2 --gas=300000000000000;
done;

# unstaking
near call $CONTRACT_NAME unstake '{"amount": "2000130228351042603620956"}' --accountId=$ID --gas=300000000000000;
near call $CONTRACT_NAME unstake_all --accountId=$ID --gas=300000000000000;

# withdraw
near call $CONTRACT_NAME withdraw '{"amount": "3000000000000000000000000"}' --accountId=$ID --gas=300000000000000;
near call $CONTRACT_NAME withdraw_all --accountId=$ID --gas=300000000000000;
