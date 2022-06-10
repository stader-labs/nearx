use near_sdk::json_types::{U128, U64};
use near_units::parse_near;
use near_x::state::{AccountResponse, Fraction, NearxPoolStateResponse, ValidatorInfoResponse};
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
use workspaces::prelude::DevAccountDeployer;
use workspaces::result::CallExecutionDetails;
use workspaces::{network::Sandbox, Account, AccountId, Contract, Worker};
// TODO - bchain - Use generic paths
const NEARX_WASM_FILEPATH: &str = "./../../res/near_x.wasm";
const STAKE_POOL_WASM: &str = "./../../res/mock_stake_pool.wasm";

pub fn get_validator_account_id(validator_idx: u32) -> AccountId {
    AccountId::from_str(format!("stake_public_key_{}", validator_idx).as_str()).unwrap()
}

pub struct IntegrationTestContext<T> {
    pub worker: Worker<T>,
    pub validator_count: u32,
    pub nearx_contract: Contract,
    pub validator_to_stake_pool_contract: HashMap<AccountId, Contract>,
    // pub stake_pool_contract: Contract,
    pub nearx_operator: Account,
    pub nearx_owner: Account,
    pub user1: Account,
    pub user2: Account,
    pub user3: Account,
}

impl IntegrationTestContext<Sandbox> {
    // Return type is the worker, nearx liquid token contract and stake pool contract with 3 users and operator, owner account
    pub async fn new(validator_count: u32) -> anyhow::Result<IntegrationTestContext<Sandbox>> {
        println!("Connecting to sandbox!");
        let worker = workspaces::sandbox().await?;
        let nearx_wasm = std::fs::read(NEARX_WASM_FILEPATH)?;
        let stake_pool_wasm = std::fs::read(STAKE_POOL_WASM)?;
        let nearx_contract = worker.dev_deploy(&nearx_wasm).await?;
        let mut validator_to_stake_pool_contract = HashMap::default();

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
                    "owner_account_id": nearx_owner.id(),
                    "operator_account_id": nearx_operator.id(),
            }))?
            .transact()
            .await?;
        println!("Initialized the Nearx pool contract!");

        // Deploy all validator stake pool contracts
        println!(
            "Deploying validator stake pool contracts for {:?}",
            validator_count
        );
        for i in 0..validator_count {
            let stake_pool_contract = worker.dev_deploy(&stake_pool_wasm).await?;
            let validator_account_id = get_validator_account_id(i);

            // Initialized the stake pool contract
            println!("Initializing the stake pool contract");
            stake_pool_contract
                .call(&worker, "new")
                .max_gas()
                .transact()
                .await?;
            println!("Initializing the stake pool contract");

            // Add the stake pool
            println!("Adding validator");
            nearx_operator
                .call(&worker, nearx_contract.id(), "add_validator")
                .args_json(json!({ "validator": stake_pool_contract.id() }))?
                .transact()
                .await?;
            println!("Successfully Added the validator!");

            validator_to_stake_pool_contract.insert(validator_account_id, stake_pool_contract);
        }

        println!("Fast forward to around 122 epochs");
        worker.fast_forward(1000).await?;

        Ok(IntegrationTestContext {
            worker,
            validator_count,
            nearx_contract,
            validator_to_stake_pool_contract,
            nearx_operator,
            nearx_owner,
            user1,
            user2,
            user3,
        })
    }

    pub fn get_stake_pool_contract(&self, validator_idx: u32) -> &Contract {
        let validator_account_id = get_validator_account_id(validator_idx);
        self.validator_to_stake_pool_contract
            .get(&validator_account_id)
            .unwrap()
    }

    pub async fn deposit_direct_stake(
        &self,
        user: &Account,
    ) -> anyhow::Result<CallExecutionDetails> {
        user.call(
            &self.worker,
            self.nearx_contract.id(),
            "deposit_and_stake_direct_stake",
        )
        .max_gas()
        .deposit(parse_near!("10 N"))
        .transact()
        .await
    }

    pub async fn deposit(&self, user: &Account) -> anyhow::Result<CallExecutionDetails> {
        user.call(&self.worker, self.nearx_contract.id(), "deposit_and_stake")
            .max_gas()
            .deposit(parse_near!("10 N"))
            .transact()
            .await
    }

    pub async fn epoch_stake(&self) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_contract
            .call(&self.worker, "epoch_stake")
            .max_gas()
            .transact()
            .await
    }

    pub async fn auto_compound_rewards(
        &self,
        validator: &AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_contract
            .call(&self.worker, "epoch_autocompound_rewards")
            .max_gas()
            .args_json(json!({ "validator": validator }))?
            .transact()
            .await
    }

    pub async fn ft_transfer(
        &self,
        sender: &Account,
        receiver: &Account,
        amount: String,
    ) -> anyhow::Result<CallExecutionDetails> {
        sender
            .call(&self.worker, self.nearx_contract.id(), "ft_transfer")
            .deposit(parse_near!("0.000000000000000000000001 N"))
            .max_gas()
            .args_json(json!({ "receiver_id": receiver.id(), "amount": amount }))?
            .transact()
            .await
    }

    pub async fn add_stake_pool_rewards(
        &self,
        amount: U128,
        stake_pool_contract: &Contract,
    ) -> anyhow::Result<CallExecutionDetails> {
        stake_pool_contract
            .call(&self.worker, "add_reward_for")
            .max_gas()
            .args_json(json!({ "amount": amount, "account_id": self.nearx_contract.id() }))?
            .transact()
            .await
    }

    pub async fn set_reward_fee(
        &self,
        reward_fee: Fraction,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(&self.worker, &self.nearx_contract.id(), "set_reward_fee")
            .max_gas()
            .args_json(
                json!({ "numerator": reward_fee.numerator, "denominator": reward_fee.denominator }),
            )?
            .transact()
            .await
    }

    pub async fn get_user_deposit(&self, user: &AccountId) -> anyhow::Result<U128> {
        Ok(self.get_user_account(user).await?.staked_balance)
    }

    pub async fn get_user_account(&self, user: &AccountId) -> anyhow::Result<AccountResponse> {
        self.nearx_contract
            .call(&self.worker, "get_account")
            .args_json(json!({ "account_id": user }))?
            .view()
            .await?
            .json::<AccountResponse>()
    }

    pub async fn get_validator_info(
        &self,
        validator: &AccountId,
    ) -> anyhow::Result<ValidatorInfoResponse> {
        self.nearx_contract
            .call(&self.worker, "get_validator_info")
            .args_json(json!({ "validator": validator }))?
            .view()
            .await?
            .json::<ValidatorInfoResponse>()
    }

    pub async fn get_user_token_balance(&self, user: &AccountId) -> anyhow::Result<U128> {
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

    pub async fn get_stake_pool_total_staked_amount(
        &self,
        stake_pool_contract: &Contract,
    ) -> anyhow::Result<U128> {
        stake_pool_contract
            .call(&self.worker, "get_account_staked_balance")
            .args_json(json!({ "account_id": self.nearx_contract.id() }))?
            .view()
            .await?
            .json::<U128>()
    }

    /*
    pub async fn get_stake_pool_total_unstaked_amount(
        &self,
        stake_pool_contract: &Contract,
    ) -> anyhow::Result<U128> {
        stake_pool_contract
            .call(&self.worker, "get_account_unstaked_balance")
            .args_json(json!({ "account_id": self.nearx_contract.id() }))?
            .view()
            .await?
            .json::<U128>()
    }
    */

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
