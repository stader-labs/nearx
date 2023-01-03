ID=staderlabs-test.near
CONTRACT_NAME=testing-token.$ID

# Create contract account
near create-account $CONTRACT_NAME --masterAccount=$ID

# Deploy the contract
near deploy $CONTRACT_NAME --wasmFile=res/near_x.wasm --initFunction=new --initArgs='{{"owner_account_id": "'$ID'", "operator_account_id": "'$ID'", "treasury_account_id": "'$ID'"}}' --initGas=300000000000000

# Init
near call $CONTRACT_NAME new '{"owner_account_id": "staderlabs.testnet", "operator_account_id": "bchain99.testnet", "treasury_account_id": "bchaina.testnet"}' --accountId=$ID