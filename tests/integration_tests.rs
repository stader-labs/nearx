mod helpers;
mod types;

use serde_json::json;
use workspaces::{Account, AccountId, Contract, Worker};

use crate::helpers::ntoy;
use crate::types::{AccountResponse, NearxPoolStateResponse, ValidatorInfoResponse};
use near_units::*;
use workspaces::network::Sandbox;
use workspaces::prelude::*;

const NEAR_LIQUID_TOKEN_WASM_FILEPATH: &str =
    "./../target/wasm32-unknown-unknown/release/near_liquid_token.wasm";
const STAKE_POOL_WASM: &str = "./..//res/staking_pool.wasm";

// Return type is the worker, nearx liquid token contract and stake pool contract with 3 users and operator, owner account
async fn setup_sandbox_workspace() -> anyhow::Result<(
    Worker<Sandbox>,
    Contract,
    Contract,
    Account,
    Account,
    Account,
    Account,
    Account,
)> {
    let worker = workspaces::sandbox().await?;
    let near_pool_wasm = std::fs::read(NEAR_LIQUID_TOKEN_WASM_FILEPATH)?;
    let stake_pool_wasm = std::fs::read(STAKE_POOL_WASM)?;
    let near_pool_contract = worker.dev_deploy(&near_pool_wasm).await?;
    let stake_pool_contract = worker.dev_deploy(&stake_pool_wasm).await?;

    let operator = worker.dev_create_account().await?;
    let owner = worker.dev_create_account().await?;

    let user1 = worker.dev_create_account().await?;
    let user2 = worker.dev_create_account().await?;
    let user3 = worker.dev_create_account().await?;

    println!("Setting up the sandbox workspace!");

    // init the near pool contract
    println!("Initializing the Nearx pool contract!");
    near_pool_contract
        .call(&worker, "new")
        .args_json(json!({
                "owner_account_id": operator.id().clone(),
                "operator_account_id": owner.id().clone(),
        }))?
        .transact()
        .await?;
    println!("Initialized the Nearx pool contract!");

    // init the stake pool contract
    println!("Initializing the stake pool contract!");
    stake_pool_contract
        .call(&worker, "new")
        .args_json(json!({
            "owner_id": stake_pool_contract.id(),
            "stake_public_key": "nDK1kgHNzu5MQaKtdCnfHmq8gGqteb4yYUKvjFkyZ3Y",
            "reward_fee_fraction": json!({
                "numerator": 10,
                "denominator": 100
            }),
        }))?
        .max_gas()
        .transact()
        .await?;
    println!("Initialized the stake pool contract!");

    // Add the stake pool
    println!("Adding validator");
    operator
        .call(&worker, near_pool_contract.id(), "add_validator")
        .args_json(json!({ "account_id": stake_pool_contract.id() }))?
        .transact()
        .await?;
    println!("Successfully Added the validator!");

    // Assert initial account stake balance is 0
    println!("Asserting that initial stake is 0");
    let stake_pool_initial_stake = stake_pool_contract
        .call(&worker, "get_account_staked_balance")
        .args_json(json!({ "account_id": near_pool_contract.id() }))?
        .view()
        .await?
        .json::<String>()?;
    assert_eq!(stake_pool_initial_stake, 0.to_string());
    println!("Assertion successful!");

    Ok((
        worker,
        near_pool_contract,
        stake_pool_contract,
        user1,
        user2,
        user3,
        operator,
        owner,
    ))
}

async fn deposit(
    worker: &Worker<Sandbox>,
    near_pool_contract: &Contract,
    user: &Account,
) -> anyhow::Result<()> {
    user.call(worker, near_pool_contract.id(), "deposit_and_stake")
        .max_gas()
        .deposit(parse_near!("10 N"))
        .transact()
        .await?;

    Ok(())
}

async fn restake_staking_pool(
    worker: &Worker<Sandbox>,
    stake_pool_contract: &Contract,
) -> anyhow::Result<()> {
    stake_pool_contract
        .call(worker, "ping")
        .max_gas()
        .transact()
        .await?;

    Ok(())
}

async fn auto_compound_rewards(
    worker: &Worker<Sandbox>,
    near_pool_contract: &Contract,
) -> anyhow::Result<()> {
    near_pool_contract
        .call(worker, "distribute_rewards")
        .max_gas()
        .args_json(json!({ "val_inx": 0 }))?
        .transact()
        .await?;

    Ok(())
}

async fn ft_transfer(
    worker: &Worker<Sandbox>,
    near_pool_contract: &Contract,
    sender: &Account,
    receiver: &Account,
    amount: String,
) -> anyhow::Result<()> {
    sender
        .call(worker, near_pool_contract.id(), "ft_transfer")
        .deposit(parse_near!("0.000000000000000000000001 N"))
        .max_gas()
        .args_json(json!({ "receiver_id": receiver.id(), "amount": amount }))?
        .transact()
        .await?;

    Ok(())
}

async fn get_user_deposit(
    worker: &Worker<Sandbox>,
    near_pool_contract: &Contract,
    user: AccountId,
) -> anyhow::Result<String> {
    let result = near_pool_contract
        .call(worker, "get_account")
        .args_json(json!({ "account_id": user }))?
        .view()
        .await?
        .json::<AccountResponse>()?;

    Ok(result.staked_balance)
}

async fn get_validator_info(
    worker: &Worker<Sandbox>,
    near_pool_contract: &Contract,
) -> anyhow::Result<ValidatorInfoResponse> {
    near_pool_contract
        .call(worker, "get_validator_info")
        .args_json(json!({ "inx": 0 }))?
        .view()
        .await?
        .json::<ValidatorInfoResponse>()
}

async fn get_user_token_balance(
    worker: &Worker<Sandbox>,
    near_pool_contract: &Contract,
    user: AccountId,
) -> anyhow::Result<String> {
    near_pool_contract
        .call(worker, "ft_balance_of")
        .args_json(json!({ "account_id": user }))?
        .view()
        .await?
        .json::<String>()
}

async fn get_nearx_price(
    worker: &Worker<Sandbox>,
    near_pool_contract: &Contract,
) -> anyhow::Result<String> {
    near_pool_contract
        .call(worker, "get_nearx_price")
        .view()
        .await?
        .json::<String>()
}

async fn get_nearx_state(
    worker: &Worker<Sandbox>,
    near_pool_contract: &Contract,
) -> anyhow::Result<NearxPoolStateResponse> {
    near_pool_contract
        .call(worker, "get_near_pool_state")
        .view()
        .await?
        .json::<NearxPoolStateResponse>()
}

async fn get_total_staked_amount(
    worker: &Worker<Sandbox>,
    near_pool_contract: &Contract,
) -> anyhow::Result<String> {
    near_pool_contract
        .call(worker, "get_total_staked")
        .view()
        .await?
        .json::<String>()
}

async fn get_stake_pool_total_staked_amount(
    worker: &Worker<Sandbox>,
    stake_pool_contract: &Contract,
    user: &AccountId,
) -> anyhow::Result<String> {
    stake_pool_contract
        .call(worker, "get_account_staked_balance")
        .args_json(json!({ "account_id": user }))?
        .view()
        .await?
        .json::<String>()
}

async fn get_total_tokens_supply(
    worker: &Worker<Sandbox>,
    near_pool_contract: &Contract,
) -> anyhow::Result<String> {
    near_pool_contract
        .call(worker, "ft_total_supply")
        .view()
        .await?
        .json::<String>()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialization
    println!("***** Step 1: Initialization *****");
    let (worker, near_pool_contract, stake_pool_contract, user1, user2, user3, operator, owner) =
        setup_sandbox_workspace().await?;

    // First test
    // user1, user2 and user3 deposit 10 NEAR each. We check whether the staking contract
    // Check initial deposits
    println!("**** Step 2: User deposit test ****");
    println!("Checking initial user deposits");

    let user1_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user1.id().clone()).await?;
    let user2_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user2.id().clone()).await?;
    let user3_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user3.id().clone()).await?;
    assert_eq!(user1_staked_amount, 0.to_string());
    assert_eq!(user2_staked_amount, 0.to_string());
    assert_eq!(user3_staked_amount, 0.to_string());

    println!("Successfully checked initial user deposits");

    let stake_pool_state = stake_pool_contract.view_account(&worker).await?;
    println!(
        "stake pool account details before deposits are {:?}",
        stake_pool_state
    );

    println!("**** Simulating user deposits ****");
    println!("User 1 depositing");
    deposit(&worker, &near_pool_contract, &user1).await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    deposit(&worker, &near_pool_contract, &user2).await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    deposit(&worker, &near_pool_contract, &user3).await?;
    println!("User 3 successfully deposited");

    println!("Checking user deposits after users have deposited");
    let user1_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user1.id().clone()).await?;
    let user2_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user2.id().clone()).await?;
    let user3_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, ntoy(10).to_string());
    assert_eq!(user2_staked_amount, ntoy(10).to_string());
    assert_eq!(user3_staked_amount, ntoy(10).to_string());

    let user1_token_balance =
        get_user_token_balance(&worker, &near_pool_contract, user1.id().clone()).await?;
    let user2_token_balance =
        get_user_token_balance(&worker, &near_pool_contract, user2.id().clone()).await?;
    let user3_token_balance =
        get_user_token_balance(&worker, &near_pool_contract, user3.id().clone()).await?;

    assert_eq!(user1_token_balance, ntoy(10).to_string());
    assert_eq!(user2_token_balance, ntoy(10).to_string());
    assert_eq!(user3_token_balance, ntoy(10).to_string());

    let nearx_price = get_nearx_price(&worker, &near_pool_contract).await?;
    assert_eq!(nearx_price, ntoy(1).to_string());

    let total_staked_amount = get_total_staked_amount(&worker, &near_pool_contract).await?;
    assert_eq!(total_staked_amount, ntoy(30).to_string());

    let stake_pool_staked_amount =
        get_stake_pool_total_staked_amount(&worker, &stake_pool_contract, near_pool_contract.id())
            .await?;
    assert_eq!(stake_pool_staked_amount, ntoy(30).to_string());

    let total_tokens_minted = get_total_tokens_supply(&worker, &near_pool_contract).await?;
    assert_eq!(total_tokens_minted, ntoy(30).to_string());

    let stake_pool_state = stake_pool_contract.view_account(&worker).await?;
    println!("stake pool account details after user deposits {:?}", stake_pool_state);

    // Second test
    // Test token transfers
    println!("**** Step 3: Token transferring ****");

    println!("Successfully checked initial user deposits");

    println!("User 1 transfers 5 tokens to User 2");
    ft_transfer(
        &worker,
        &near_pool_contract,
        &user1,
        &user2,
        ntoy(5).to_string(),
    )
        .await?;
    println!("User 2 transfers 3 tokens to User 3");
    ft_transfer(
        &worker,
        &near_pool_contract,
        &user2,
        &user3,
        ntoy(3).to_string(),
    )
        .await?;
    println!("User 3 transfers 1 token to User 1");
    ft_transfer(
        &worker,
        &near_pool_contract,
        &user3,
        &user1,
        ntoy(1).to_string(),
    )
        .await?;

    println!("Checking user deposits after users have deposited");
    let user1_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user1.id().clone()).await?;
    let user2_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user2.id().clone()).await?;
    let user3_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, ntoy(6).to_string());
    assert_eq!(user2_staked_amount, ntoy(12).to_string());
    assert_eq!(user3_staked_amount, ntoy(12).to_string());

    let user1_token_balance =
        get_user_token_balance(&worker, &near_pool_contract, user1.id().clone()).await?;
    let user2_token_balance =
        get_user_token_balance(&worker, &near_pool_contract, user2.id().clone()).await?;
    let user3_token_balance =
        get_user_token_balance(&worker, &near_pool_contract, user3.id().clone()).await?;

    assert_eq!(user1_token_balance, ntoy(6).to_string());
    assert_eq!(user2_token_balance, ntoy(12).to_string());
    assert_eq!(user3_token_balance, ntoy(12).to_string());

    let nearx_price = get_nearx_price(&worker, &near_pool_contract).await?;
    assert_eq!(nearx_price, ntoy(1).to_string());

    let total_staked_amount = get_total_staked_amount(&worker, &near_pool_contract).await?;
    assert_eq!(total_staked_amount, ntoy(30).to_string());

    let stake_pool_staked_amount =
        get_stake_pool_total_staked_amount(&worker, &stake_pool_contract, near_pool_contract.id())
            .await?;
    assert_eq!(stake_pool_staked_amount, ntoy(30).to_string());

    let total_tokens_minted = get_total_tokens_supply(&worker, &near_pool_contract).await?;
    assert_eq!(total_tokens_minted, ntoy(30).to_string());

    println!("**** Step 4: Auto compounding ****");

    // TODO - bchain - validators on sandbox don't generate rewards nor do user level staking. Check with NEAR team
    println!("Fast forward 100400 blocks");
    worker.fast_forward(61400).await?;

    println!("Auto compounding stake pool");
    user1
        .transfer_near(&worker, stake_pool_contract.id(), ntoy(10))
        .await?;

    let operator_account_details = operator.view_account(&worker).await?;
    println!(
        "operator_account before auto compounding is {:?}",
        operator_account_details
    );

    let stake_pool_state = stake_pool_contract.view_account(&worker).await?;
    println!("stake pool account details are {:?}", stake_pool_state);

    // restake_staking_pool(&worker, &stake_pool_contract).await?;
    let nearx_state = get_nearx_state(&worker, &near_pool_contract).await?;
    println!("nearx_state before auto compounding is {:?}", nearx_state);

    println!("Auto compounding nearx pool");
    auto_compound_rewards(&worker, &near_pool_contract).await?;

    let nearx_price = get_nearx_price(&worker, &near_pool_contract).await?;
    println!("nearx_price is {:?}", nearx_price);

    let total_staked_amount = get_total_staked_amount(&worker, &near_pool_contract).await?;
    println!("total_staked amount is {:?}", total_staked_amount);

    let total_tokens_minted = get_total_tokens_supply(&worker, &near_pool_contract).await?;
    assert_eq!(total_tokens_minted, ntoy(30).to_string());

    let user1_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user1.id().clone()).await?;
    let user2_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user2.id().clone()).await?;
    let user3_staked_amount =
        get_user_deposit(&worker, &near_pool_contract, user3.id().clone()).await?;

    println!("user1_staked_amount is {:?}", user1_staked_amount);
    println!("user2_staked_amount is {:?}", user2_staked_amount);
    println!("user3_staked_amount is {:?}", user3_staked_amount);

    let user1_token_balance =
        get_user_token_balance(&worker, &near_pool_contract, user1.id().clone()).await?;
    let user2_token_balance =
        get_user_token_balance(&worker, &near_pool_contract, user2.id().clone()).await?;
    let user3_token_balance =
        get_user_token_balance(&worker, &near_pool_contract, user3.id().clone()).await?;

    assert_eq!(user1_token_balance, ntoy(6).to_string());
    assert_eq!(user2_token_balance, ntoy(12).to_string());
    assert_eq!(user3_token_balance, ntoy(12).to_string());

    let validator = get_validator_info(&worker, &near_pool_contract).await?;
    println!("validator is {:?}", validator);

    let operator_account_details = operator.view_account(&worker).await?;
    println!(
        "operator_account after auto compounding is {:?}",
        operator_account_details
    );

    let stake_pool_staked_amount =
        get_stake_pool_total_staked_amount(&worker, &stake_pool_contract, near_pool_contract.id())
            .await?;
    println!(
        "Amount staked with stake pool is {:?}",
        stake_pool_staked_amount
    );

    let nearx_state = get_nearx_state(&worker, &near_pool_contract).await?;
    println!("nearx_state after auto compounding is {:?}", nearx_state);

    Ok(())
}
