near call $CONTRACT_NAME drain_unstake '{"validator": "omnistake_v5.factory01.littlefarm.testnet"}' --accountId=$ID --gas=300000000000000;

near call $CONTRACT_NAME drain_withdraw '{"validator": "omnistake_v5.factory01.littlefarm.testnet"}' --accountId=$ID --gas=300000000000000;