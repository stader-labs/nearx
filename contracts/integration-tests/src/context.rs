use crate::helpers::ntoy;
use near_sdk::json_types::{U128, U64};
use near_units::parse_near;
use near_x::constants::NUM_EPOCHS_TO_UNLOCK;
use near_x::state::{
    AccountResponse, Fraction, HumanReadableAccount, NearxPoolStateResponse, ValidatorInfoResponse,
};
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
use workspaces::prelude::DevAccountDeployer;
use workspaces::result::CallExecutionDetails;
use workspaces::{network::Sandbox, prelude::*, Account, AccountId, Contract, Worker};

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
    pub nearx_treasury: Account,
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
        let nearx_treasury = worker.dev_create_account().await?;

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
                    "treasury_account_id": nearx_treasury.id().clone()
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

            println!("Adding validator {:?}", stake_pool_contract.id());
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

        for i in 0..validator_count {
            println!("Seeding with manager deposit of 5N");
            let res = nearx_owner
                .call(&worker, nearx_contract.id(), "manager_deposit_and_stake")
                .max_gas()
                .deposit(parse_near!("5 N"))
                .transact()
                .await?;
            println!("Seed with manager deposit of 5N");
        }

        println!("Fast forward to around 10 epochs");
        worker.fast_forward(10000).await?;

        Ok(IntegrationTestContext {
            worker,
            validator_count,
            nearx_contract,
            validator_to_stake_pool_contract,
            nearx_operator,
            nearx_owner,
            nearx_treasury,
            user1,
            user2,
            user3,
        })
    }

    pub async fn run_epoch_methods(&self) -> anyhow::Result<()> {
        let current_epoch = self.get_current_epoch().await?;

        // Run the autocompounding epoch
        for i in 0..self.validator_count {
            self.auto_compound_rewards(self.get_stake_pool_contract(i).id())
                .await?;
        }

        // Run the staking epoch
        self.epoch_stake().await?;

        // Run the unstaking epoch
        let mut res = true;
        while res {
            let output = self.epoch_unstake().await?;
            println!("output of epoch unstake is {:?}", output);
            res = output.json::<bool>().unwrap();
        }

        // Run the withdraw epoch
        for i in 0..self.validator_count {
            let validator_info = self
                .get_validator_info(self.get_stake_pool_contract(i).id().clone())
                .await?;

            if validator_info.unstaked.0 != 0
                && validator_info.last_unstake_start_epoch.0 + NUM_EPOCHS_TO_UNLOCK
                    < current_epoch.0
            {
                self.epoch_withdraw(self.get_stake_pool_contract(i).id().clone())
                    .await?;
            }
        }

        // Run the validator balance syncing epoch
        for i in 0..self.validator_count {
            self.sync_validator_balances(self.get_stake_pool_contract(i).id().clone())
                .await?;
        }

        Ok(())
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
        amount: u128,
    ) -> anyhow::Result<CallExecutionDetails> {
        user.call(
            &self.worker,
            self.nearx_contract.id(),
            "deposit_and_stake_direct_stake",
        )
        .max_gas()
        .deposit(amount)
        .transact()
        .await
    }

    pub async fn deposit(
        &self,
        user: &Account,
        amount: u128,
    ) -> anyhow::Result<CallExecutionDetails> {
        user.call(&self.worker, self.nearx_contract.id(), "deposit_and_stake")
            .max_gas()
            .deposit(amount)
            .transact()
            .await
    }

    pub async fn unstake(
        &self,
        user: &Account,
        amount: U128,
    ) -> anyhow::Result<CallExecutionDetails> {
        user.call(&self.worker, self.nearx_contract.id(), "unstake")
            .max_gas()
            .args_json(json!({ "amount": amount }))?
            .transact()
            .await
    }

    pub async fn withdraw_all(&self, user: &Account) -> anyhow::Result<CallExecutionDetails> {
        user.call(&self.worker, self.nearx_contract.id(), "withdraw_all")
            .max_gas()
            .transact()
            .await
    }

    pub async fn withdraw(
        &self,
        user: &Account,
        amount: U128,
    ) -> anyhow::Result<CallExecutionDetails> {
        user.call(&self.worker, self.nearx_contract.id(), "withdraw")
            .max_gas()
            .args_json(json!({ "amount": amount }))?
            .transact()
            .await
    }

    pub async fn pause_validator(
        &self,
        validator: &AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(&self.worker, self.nearx_contract.id(), "pause_validator")
            .max_gas()
            .args_json(json!({ "validator": validator }))?
            .transact()
            .await
    }

    pub async fn remove_validator(
        &self,
        validator: &AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(&self.worker, self.nearx_contract.id(), "remove_validator")
            .max_gas()
            .args_json(json!({ "validator": validator }))?
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

    pub async fn epoch_unstake(&self) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_contract
            .call(&self.worker, "epoch_unstake")
            .max_gas()
            .transact()
            .await
    }

    pub async fn drain_unstake(
        &self,
        validator: AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(&self.worker, self.nearx_contract.id(), "drain_unstake")
            .max_gas()
            .args_json(json!({ "validator": validator }))?
            .transact()
            .await
    }

    pub async fn epoch_withdraw(
        &self,
        validator: AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_contract
            .call(&self.worker, "epoch_withdraw")
            .max_gas()
            .args_json(json!({ "validator": validator }))?
            .transact()
            .await
    }

    pub async fn drain_withdraw(
        &self,
        validator: AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(&self.worker, self.nearx_contract.id(), "drain_withdraw")
            .max_gas()
            .args_json(json!({ "validator": validator }))?
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
            .args_json(json!({ "validator": validator.clone() }))?
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

    pub async fn adjust_balance(
        &self,
        stake_pool_contract: &AccountId,
        staked_delta: U128,
        unstaked_delta: U128,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner.call(&self.worker, stake_pool_contract, "adjust_balance").max_gas()
            .args_json(json!({ "account_id": stake_pool_contract, "staked_delta": staked_delta.0, "unstaked_delta": unstaked_delta.0 }))?
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
            .args_json(json!({ "amount": amount, "account_id": self.nearx_contract.id().clone() }))?
            .transact()
            .await
    }

    pub async fn ping_stake_pool_contract(
        &self,
        stake_pool_contract: &Contract,
        amount: U128,
    ) -> anyhow::Result<CallExecutionDetails> {
        stake_pool_contract
            .call(&self.worker, "ping")
            .args_json(json!({
                "amount": amount,
                "account_id": self.nearx_contract.id().clone()
            }))?
            .max_gas()
            .transact()
            .await
    }

    pub async fn set_stake_pool_panic(
        &self,
        stake_pool_contract: &AccountId,
        panic: bool,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(&self.worker, stake_pool_contract, "set_panic")
            .max_gas()
            .args_json(json!({ "panic": panic }))?
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
            .deposit(1)
            .args_json(
                json!({ "numerator": reward_fee.numerator, "denominator": reward_fee.denominator }),
            )?
            .transact()
            .await
    }

    pub async fn sync_validator_balances(
        &self,
        validator_id: AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(
                &self.worker,
                &self.nearx_contract.id(),
                "sync_balance_from_validator",
            )
            .max_gas()
            .args_json(json!({ "validator_id": validator_id }))?
            .transact()
            .await
    }

    pub async fn get_stake_pool_accounts(
        &self,
        stake_pool_contract: &Contract,
    ) -> anyhow::Result<Vec<HumanReadableAccount>> {
        stake_pool_contract
            .call(&self.worker, "get_accounts")
            .args_json(json!({ "from_index": 0, "limit": 10 }))?
            .view()
            .await?
            .json::<Vec<HumanReadableAccount>>()
    }

    pub async fn get_stake_pool_total_staked_balance(
        &self,
        stake_pool_contract: &Contract,
    ) -> anyhow::Result<U128> {
        stake_pool_contract
            .call(&self.worker, "get_total_staked_balance")
            .view()
            .await?
            .json::<U128>()
    }

    #[deprecated]
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

    pub async fn get_user_account(&self, user: AccountId) -> anyhow::Result<AccountResponse> {
        self.nearx_contract
            .call(&self.worker, "get_account")
            .args_json(json!({ "account_id": user }))?
            .view()
            .await?
            .json::<AccountResponse>()
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

    pub async fn get_stake_pool_total_staked_amount(
        &self,
        stake_pool_contract: &Contract,
    ) -> anyhow::Result<U128> {
        stake_pool_contract
            .call(&self.worker, "get_account_staked_balance")
            .args_json(json!({ "account_id": self.nearx_contract.id().clone() }))?
            .view()
            .await?
            .json::<U128>()
    }

    pub async fn get_stake_pool_total_unstaked_amount(
        &self,
        stake_pool_contract: &Contract,
    ) -> anyhow::Result<U128> {
        stake_pool_contract
            .call(&self.worker, "get_account_unstaked_balance")
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

    pub async fn is_validator_unstake_pending(&self, validator: AccountId) -> anyhow::Result<bool> {
        self.nearx_contract
            .call(&self.worker, "is_validator_unstake_pending")
            .args_json(json!({ "validator": validator }))?
            .view()
            .await?
            .json::<bool>()
    }

    pub async fn get_validators(&self) -> anyhow::Result<Vec<ValidatorInfoResponse>> {
        self.nearx_operator
            .call(&self.worker, self.nearx_contract.id(), "get_validators")
            .view()
            .await?
            .json::<Vec<ValidatorInfoResponse>>()
    }
}
