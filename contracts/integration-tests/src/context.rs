use near_sdk::json_types::{U128, U64};
use near_units::parse_near;
use near_x::state::{AccountResponse, Fraction, NearxPoolStateResponse, ValidatorInfoResponse};
use serde_json::json;
use workspaces::prelude::DevAccountDeployer;
use workspaces::{network::Sandbox, Account, AccountId, Contract, Worker};

// TODO - bchain - Use generic paths
const NEARX_WASM_FILEPATH: &str =
    "/Users/bharath12345/stader-work/near-liquid-token/res/near_x.wasm";
const STAKE_POOL_WASM: &str =
    "/Users/bharath12345/stader-work/near-liquid-token/res/mock_stake_pool.wasm";

pub struct IntegrationTestContext<T> {
    pub worker: Worker<T>,
    pub nearx_contract: Contract,
    pub stake_pool_contract: Contract,
    pub nearx_operator: Account,
    pub nearx_owner: Account,
    pub user1: Account,
    pub user2: Account,
    pub user3: Account,
}

impl IntegrationTestContext<Sandbox> {
    // Return type is the worker, nearx liquid token contract and stake pool contract with 3 users and operator, owner account
    // TODO - Take number of validators as parameters
    pub async fn new() -> anyhow::Result<IntegrationTestContext<Sandbox>> {
        println!("Connecting to sandbox!");
        let worker = workspaces::sandbox().await?;
        let nearx_wasm = std::fs::read(NEARX_WASM_FILEPATH)?;
        let stake_pool_wasm = std::fs::read(STAKE_POOL_WASM)?;
        let nearx_contract = worker.dev_deploy(&nearx_wasm).await?;
        let stake_pool_contract = worker.dev_deploy(&stake_pool_wasm).await?;

        let nearx_operator = worker.dev_create_account().await?;
        let nearx_owner = worker.dev_create_account().await?;

        let user1 = worker.dev_create_account().await?;
        let user2 = worker.dev_create_account().await?;
        let user3 = worker.dev_create_account().await?;

        println!("Setting up the sandbox workspace!");

        // init the near pool contract
        println!("Initializing the Nearx pool contract!");
        nearx_contract
            .call(&worker, "new")
            .args_json(json!({
                    "owner_account_id": nearx_owner.id().clone(),
                    "operator_account_id": nearx_operator.id().clone(),
            }))?
            .transact()
            .await?;
        println!("Initialized the Nearx pool contract!");

        // init the stake pool contract
        println!("Initializing the stake pool contract!");
        stake_pool_contract
            .call(&worker, "new")
            .max_gas()
            .transact()
            .await?;
        println!("Initialized the stake pool contract!");

        // Add the stake pool
        println!("Adding validator");
        nearx_operator
            .call(&worker, nearx_contract.id(), "add_validator")
            .args_json(json!({ "validator": stake_pool_contract.id() }))?
            .transact()
            .await?;
        println!("Successfully Added the validator!");

        // Assert initial account stake balance is 0
        println!("Asserting that initial stake is 0");
        let stake_pool_initial_stake = stake_pool_contract
            .call(&worker, "get_account_staked_balance")
            .args_json(json!({ "account_id": nearx_contract.id() }))?
            .view()
            .await?
            .json::<U128>()?;
        assert_eq!(stake_pool_initial_stake, U128(0));
        println!("Assertion successful!");

        Ok(IntegrationTestContext {
            worker,
            nearx_contract,
            stake_pool_contract,
            nearx_operator,
            nearx_owner,
            user1,
            user2,
            user3,
        })
    }

    pub async fn deposit(&self, user: &Account) -> anyhow::Result<()> {
        user.call(&self.worker, self.nearx_contract.id(), "deposit_and_stake")
            .max_gas()
            .deposit(parse_near!("10 N"))
            .transact()
            .await?;

        Ok(())
    }

    pub async fn auto_compound_rewards(&self, validator: &AccountId) -> anyhow::Result<()> {
        self.nearx_contract
            .call(&self.worker, "autocompound_rewards")
            .max_gas()
            .args_json(json!({ "validator": validator.clone() }))?
            .transact()
            .await?;

        Ok(())
    }

    pub async fn ft_transfer(
        &self,
        sender: &Account,
        receiver: &Account,
        amount: String,
    ) -> anyhow::Result<()> {
        sender
            .call(&self.worker, self.nearx_contract.id(), "ft_transfer")
            .deposit(parse_near!("0.000000000000000000000001 N"))
            .max_gas()
            .args_json(json!({ "receiver_id": receiver.id(), "amount": amount }))?
            .transact()
            .await?;

        Ok(())
    }

    pub async fn add_stake_pool_rewards(&self, amount: U128) -> anyhow::Result<()> {
        self.stake_pool_contract
            .call(&self.worker, "add_reward_for")
            .max_gas()
            .args_json(json!({ "amount": amount, "account_id": self.nearx_contract.id().clone() }))?
            .transact()
            .await?;

        Ok(())
    }

    pub async fn set_reward_fee(&self, reward_fee: Fraction) -> anyhow::Result<()> {
        self.nearx_owner
            .call(&self.worker, &self.nearx_contract.id(), "set_reward_fee")
            .max_gas()
            .args_json(
                json!({ "numerator": reward_fee.numerator, "denominator": reward_fee.denominator }),
            )?
            .transact()
            .await?;

        Ok(())
    }

    pub async fn get_user_deposit(&self, user: AccountId) -> anyhow::Result<U128> {
        let result = self
            .nearx_contract
            .call(&self.worker, "get_account")
            .args_json(json!({ "account_id": user }))?
            .view()
            .await?
            .json::<AccountResponse>()?;

        Ok(result.staked_balance)
    }

    pub async fn get_validator_info(
        &self,
        validator: AccountId,
    ) -> anyhow::Result<ValidatorInfoResponse> {
        self.nearx_contract
            .call(&self.worker, "get_validator_info")
            .args_json(json!({ "validator": validator }))?
            .view()
            .await?
            .json::<ValidatorInfoResponse>()
    }

    pub async fn get_user_token_balance(&self, user: AccountId) -> anyhow::Result<U128> {
        self.nearx_contract
            .call(&self.worker, "ft_balance_of")
            .args_json(json!({ "account_id": user }))?
            .view()
            .await?
            .json::<U128>()
    }

    pub async fn get_nearx_price(&self) -> anyhow::Result<U128> {
        self.nearx_contract
            .call(&self.worker, "get_nearx_price")
            .view()
            .await?
            .json::<U128>()
    }

    pub async fn get_nearx_state(&self) -> anyhow::Result<NearxPoolStateResponse> {
        self.nearx_contract
            .call(&self.worker, "get_nearx_pool_state")
            .view()
            .await?
            .json::<NearxPoolStateResponse>()
    }

    pub async fn get_total_staked_amount(&self) -> anyhow::Result<U128> {
        self.nearx_contract
            .call(&self.worker, "get_total_staked")
            .view()
            .await?
            .json::<U128>()
    }

    pub async fn get_stake_pool_total_staked_amount(&self) -> anyhow::Result<U128> {
        self.stake_pool_contract
            .call(&self.worker, "get_account_staked_balance")
            .args_json(json!({ "account_id": self.nearx_contract.id().clone() }))?
            .view()
            .await?
            .json::<U128>()
    }

    pub async fn get_total_tokens_supply(&self) -> anyhow::Result<U128> {
        self.nearx_contract
            .call(&self.worker, "ft_total_supply")
            .view()
            .await?
            .json::<U128>()
    }

    pub async fn get_current_epoch(&self) -> anyhow::Result<U64> {
        self.nearx_contract
            .call(&self.worker, "get_current_epoch")
            .view()
            .await?
            .json::<U64>()
    }

    pub async fn get_reward_fee(&self) -> anyhow::Result<Fraction> {
        self.nearx_contract
            .call(&self.worker, "get_reward_fee_fraction")
            .view()
            .await?
            .json::<Fraction>()
    }
}
