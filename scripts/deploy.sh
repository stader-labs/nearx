ID="<Enter sub account ID>"
CONTRACT_NAME="<Enter Contract name>".$ID

# Create contract account
near create-account $CONTRACT_NAME --masterAccount=$ID

# Deploy the contract
near deploy $CONTRACT_NAME --wasmFile=res/near_x.wasm

# Init
near call $CONTRACT_NAME new '{"owner_account_id": "'"$ID"'", "operator_account_id": "'"$ID"'", "treasury_account_id": "'"$ID"'"}' --accountId=$ID