use crate::constants::ONE_EPOCH;
use crate::helpers::ntoy;
use crate::legacy_types::{LegacyNearxPoolStateResponse, LegacyRolesResponse};
use near_sdk::json_types::{U128, U64};
use near_units::parse_near;
use near_x::constants::NUM_EPOCHS_TO_UNLOCK;
use near_x::contract::OperationControls;
use near_x::state::{
    AccountResponse, Fraction, HumanReadableAccount, NearxPoolStateResponse,
    OperationsControlUpdateRequest, RolesResponse, ValidatorInfoResponse,
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
    pub nearx_operator: Account,
    pub nearx_owner: Account,
    pub nearx_treasury: Account,
    pub user1: Account,
    pub user2: Account,
    pub user3: Account,
}

impl IntegrationTestContext<Sandbox> {
    // Return type is the worker, nearx liquid token contract and stake pool contract with 3 users and operator, owner account
    pub async fn new(
        validator_count: u32,
        nearx_wasm_file: Option<&str>,
    ) -> anyhow::Result<IntegrationTestContext<Sandbox>> {
        println!("Connecting to sandbox!");
        let worker = workspaces::sandbox().await?;
        let nearx_wasm = std::fs::read(nearx_wasm_file.unwrap_or(NEARX_WASM_FILEPATH))?;
        let stake_pool_wasm = std::fs::read(STAKE_POOL_WASM)?;
        let nearx_contract = worker.dev_deploy(&nearx_wasm).await?;
        let mut validator_to_stake_pool_contract = HashMap::default();

        let nearx_operator = worker.dev_create_account().await?;
        let nearx_owner = worker.dev_create_account().await?;
        let nearx_treasury = worker.dev_create_account().await?;

        println!("nearx_operator is {:?}", nearx_operator.id().clone());
        println!("nearx_owner is {:?}", nearx_owner.id().clone());
        println!("nearx_treasury is {:?}", nearx_treasury.id().clone());

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
            // Initially give all validators equal weight
            println!("Adding validator");
            let res = nearx_operator
                .call(&worker, nearx_contract.id(), "add_validator")
                .deposit(1)
                .args_json(json!({ "validator": stake_pool_contract.id() , "weight": 10 }))?
                .transact()
                .await?;
            println!("add_validator res is {:?}", res);
            println!("Successfully Added the validator!");

            validator_to_stake_pool_contract.insert(validator_account_id, stake_pool_contract);
        }

        for i in 0..validator_count {
            println!("Seeding with manager deposit of 5N");
            let res = nearx_owner
                .call(&worker, nearx_contract.id(), "manager_deposit_and_stake")
                .args_json(json!({ "validator": validator_to_stake_pool_contract.get(&get_validator_account_id(i)).unwrap().id().clone() }))?
                .max_gas()
                .deposit(parse_near!("5 N"))
                .transact()
                .await?;
            println!("Seed with manager deposit of 5N");
        }

        println!("Fast forward to around 10 epochs");
        worker.fast_forward(10 * ONE_EPOCH).await?;

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

    pub async fn update_operation_controls(
        &mut self,
        operations_control: OperationsControlUpdateRequest,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(
                &self.worker,
                self.nearx_contract.id(),
                "update_operations_control",
            )
            .deposit(1)
            .args_json(json!({
                "update_operations_control_request": operations_control
            }))?
            .transact()
            .await
    }

    pub async fn update_validator(
        &mut self,
        account_id: AccountId,
        weight: u16,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_operator
            .call(&self.worker, self.nearx_contract.id(), "update_validator")
            .deposit(1)
            .args_json(json!({ "validator": account_id, "weight": weight }))?
            .transact()
            .await
    }

    pub async fn add_validator(&mut self, weight: u16) -> anyhow::Result<()> {
        let new_validator_id = self.validator_count;
        self.validator_count += 1;
        let stake_pool_wasm = std::fs::read(STAKE_POOL_WASM)?;

        let stake_pool_contract = self.worker.dev_deploy(&stake_pool_wasm).await?;
        let validator_account_id = get_validator_account_id(new_validator_id);
        // Initialized the stake pool contract
        println!("Initializing the stake pool contract");
        stake_pool_contract
            .call(&self.worker, "new")
            .max_gas()
            .transact()
            .await?;

        self.nearx_operator
            .call(&self.worker, self.nearx_contract.id(), "add_validator")
            .deposit(1)
            .args_json(json!({ "validator": stake_pool_contract.id(), "weight": weight}))?
            .transact()
            .await?;

        self.validator_to_stake_pool_contract
            .insert(validator_account_id, stake_pool_contract);

        Ok(())
    }

    pub async fn run_epoch_methods(&self) -> anyhow::Result<()> {
        let current_epoch = self.get_current_epoch().await?;

        println!("Running epoch methods!");

        let MAX_LOOP_COUNT: u32 = 3 * self.validator_count;

        // Run the autocompounding epoch
        for i in 0..self.validator_count {
            let res = self
                .autocompounding_epoch(self.get_stake_pool_contract(i).id())
                .await;
            if res.is_err() {
                continue;
            }
            println!("autocompounding logs are {:?}", res.unwrap().logs());
        }

        // Run the staking epoch
        let mut res = true;
        let mut i = 0;
        while res {
            let output = self.staking_epoch().await;
            if output.is_err() {
                println!("epoch stake errored out!");
                break;
            }
            println!("epoch_stake output is {:?}", output.as_ref().unwrap());
            res = output.unwrap().json::<bool>().unwrap();
            i += 1;
            if i >= MAX_LOOP_COUNT {
                break;
            }
        }

        // Run the unstaking epoch
        let mut res = true;
        let mut i = 0;
        while res {
            let output = self.unstaking_epoch().await;
            if output.is_err() {
                println!("epoch unstake errored out!");
                break;
            }
            println!("output of epoch unstake is {:?}", output);
            res = output.unwrap().json::<bool>().unwrap();
            i += 1;
            if i >= MAX_LOOP_COUNT {
                break;
            }
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
                let res = self
                    .withdraw_epoch(self.get_stake_pool_contract(i).id().clone())
                    .await;
                if res.is_err() {
                    continue;
                }
            }
        }

        Ok(())
    }

    pub async fn update_rewards_buffer(
        &self,
        amount: u128,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(
                &self.worker,
                self.nearx_contract.id(),
                "update_rewards_buffer",
            )
            .deposit(amount)
            .max_gas()
            .transact()
            .await
    }

    pub async fn set_owner(&self, new_owner: &AccountId) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(&self.worker, self.nearx_contract.id(), "set_owner")
            .args_json(json!({ "new_owner": new_owner.clone() }))?
            .deposit(1)
            .max_gas()
            .transact()
            .await
    }

    pub async fn commit_owner(&self, new_owner: &Account) -> anyhow::Result<CallExecutionDetails> {
        new_owner
            .call(&self.worker, self.nearx_contract.id(), "commit_owner")
            .deposit(1)
            .max_gas()
            .transact()
            .await
    }

    pub async fn set_operator(
        &self,
        new_operator: &AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(&self.worker, self.nearx_contract.id(), "set_operator_id")
            .args_json(json!({ "new_operator_account_id": new_operator.clone() }))?
            .deposit(1)
            .max_gas()
            .transact()
            .await
    }

    pub async fn commit_operator(
        &self,
        new_operator: &Account,
    ) -> anyhow::Result<CallExecutionDetails> {
        new_operator
            .call(&self.worker, self.nearx_contract.id(), "commit_operator_id")
            .deposit(1)
            .max_gas()
            .transact()
            .await
    }

    pub async fn set_treasury(
        &self,
        new_treasury_id: &AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(&self.worker, self.nearx_contract.id(), "set_treasury_id")
            .args_json(json!({ "new_treasury_account_id": new_treasury_id.clone() }))?
            .deposit(1)
            .max_gas()
            .transact()
            .await
    }

    pub async fn commit_treasury(
        &self,
        new_treasury_id: &Account,
    ) -> anyhow::Result<CallExecutionDetails> {
        new_treasury_id
            .call(&self.worker, self.nearx_contract.id(), "commit_treasury_id")
            .deposit(1)
            .max_gas()
            .transact()
            .await
    }

    pub fn get_stake_pool_contract(&self, validator_idx: u32) -> &Contract {
        let validator_account_id = get_validator_account_id(validator_idx);
        self.validator_to_stake_pool_contract
            .get(&validator_account_id)
            .unwrap()
    }

    pub async fn ft_transfer_call(
        &self,
        user: &Account,
        receiving_contract: &Contract,
        amount: U128,
    ) -> anyhow::Result<CallExecutionDetails> {
        user.call(&self.worker, self.nearx_contract.id(), "ft_transfer_call")
            .args_json(json!({ "receiver_id": receiving_contract.id().clone(), "amount": amount, "msg": amount.0.to_string() }))?
            .deposit(1)
            .max_gas()
            .transact()
            .await
    }

    pub async fn deposit(
        &self,
        user: &Account,
        amount: u128,
    ) -> anyhow::Result<CallExecutionDetails> {
        user.call(&self.worker, self.nearx_contract.id(), "storage_deposit")
            .max_gas()
            .args_json(json!({}))?
            .deposit(3000000000000000000000)
            .transact()
            .await?;

        user.call(&self.worker, self.nearx_contract.id(), "deposit_and_stake")
            .max_gas()
            .deposit(amount)
            .transact()
            .await
    }

    pub async fn manager_deposit_and_stake(
        &self,
        amount: u128,
        stake_pool_contract: AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(
                &self.worker,
                self.nearx_contract.id(),
                "manager_deposit_and_stake",
            )
            .args_json(json!({ "validator": stake_pool_contract }))?
            .max_gas()
            .deposit(amount)
            .transact()
            .await
    }

    pub async fn migrate_stake_to_validator(
        &self,
        validator: AccountId,
        amount: U128,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(
                &self.worker,
                self.nearx_contract.id(),
                "migrate_stake_to_validator",
            )
            .args_json(json!({ "validator": validator, "amount": amount }))?
            .max_gas()
            .deposit(1)
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
            .deposit(1)
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
            .deposit(1)
            .max_gas()
            .args_json(json!({ "validator": validator }))?
            .transact()
            .await
    }

    pub async fn staking_epoch(&self) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_contract
            .call(&self.worker, "staking_epoch")
            .max_gas()
            .transact()
            .await
    }

    pub async fn unstaking_epoch(&self) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_contract
            .call(&self.worker, "unstaking_epoch")
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

    pub async fn withdraw_epoch(
        &self,
        validator: AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_contract
            .call(&self.worker, "withdraw_epoch")
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

    pub async fn autocompounding_epoch(
        &self,
        validator: &AccountId,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_contract
            .call(&self.worker, "autocompounding_epoch")
            .max_gas()
            .args_json(json!({ "validator": validator.clone() }))?
            .transact()
            .await
    }

    pub async fn upgrade(&self, code: Vec<u8>) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(&self.worker, &self.nearx_contract.id(), "upgrade")
            .max_gas()
            .args(code)
            .transact()
            .await
    }

    pub async fn post_upgrade_function(&self) -> anyhow::Result<String> {
        self.nearx_contract
            .call(&self.worker, "test_post_upgrade")
            .view()
            .await?
            .json::<String>()
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

    pub async fn set_refund_amount(
        &self,
        amount: U128,
        stake_pool_contract: &Contract,
    ) -> anyhow::Result<CallExecutionDetails> {
        stake_pool_contract
            .call(&self.worker, "set_refund_amount")
            .max_gas()
            .args_json(json!({ "amount": amount }))?
            .transact()
            .await
    }

    pub async fn add_min_storage_reserve(
        &self,
        amount: U128,
    ) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(
                &self.worker,
                &self.nearx_contract.id(),
                "add_min_storage_reserve",
            )
            .max_gas()
            .deposit(amount.0)
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

    pub async fn commit_reward_fee(&self) -> anyhow::Result<CallExecutionDetails> {
        self.nearx_owner
            .call(&self.worker, &self.nearx_contract.id(), "commit_reward_fee")
            .max_gas()
            .deposit(1)
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

    pub async fn get_total_validator_weight(&self) -> anyhow::Result<u16> {
        self.nearx_contract
            .call(&self.worker, "get_total_validator_weight")
            .view()
            .await?
            .json::<u16>()
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
            .call(&self.worker, "get_user_account")
            .args_json(json!({ "account_id": user }))?
            .view()
            .await?
            .json::<AccountResponse>()?;

        Ok(result.staked_balance)
    }

    pub async fn get_user_account(&self, user: AccountId) -> anyhow::Result<AccountResponse> {
        self.nearx_contract
            .call(&self.worker, "get_user_account")
            .args_json(json!({ "account_id": user }))?
            .view()
            .await?
            .json::<AccountResponse>()
    }

    pub async fn get_legacy_user_account(
        &self,
        user: AccountId,
    ) -> anyhow::Result<AccountResponse> {
        self.nearx_contract
            .call(&self.worker, "get_account")
            .args_json(json!({ "account_id": user }))?
            .view()
            .await?
            .json::<AccountResponse>()
    }

    pub async fn get_account(&self, user: AccountId) -> anyhow::Result<HumanReadableAccount> {
        self.nearx_contract
            .call(&self.worker, "get_account")
            .args_json(json!({ "account_id": user }))?
            .view()
            .await?
            .json::<HumanReadableAccount>()
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

    pub async fn get_legacy_nearx_state(&self) -> anyhow::Result<LegacyNearxPoolStateResponse> {
        self.nearx_contract
            .call(&self.worker, "get_nearx_pool_state")
            .view()
            .await?
            .json::<LegacyNearxPoolStateResponse>()
    }

    pub async fn get_total_staked_amount(&self) -> anyhow::Result<U128> {
        self.nearx_contract
            .call(&self.worker, "get_total_staked_balance")
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

    pub async fn get_roles(&self) -> anyhow::Result<RolesResponse> {
        self.nearx_operator
            .call(&self.worker, self.nearx_contract.id(), "get_roles")
            .view()
            .await?
            .json::<RolesResponse>()
    }

    pub async fn get_legacy_roles(&self) -> anyhow::Result<LegacyRolesResponse> {
        self.nearx_operator
            .call(&self.worker, self.nearx_contract.id(), "get_roles")
            .view()
            .await?
            .json::<LegacyRolesResponse>()
    }

    pub async fn get_operations_controls(&self) -> anyhow::Result<OperationControls> {
        self.nearx_operator
            .call(
                &self.worker,
                self.nearx_contract.id(),
                "get_operations_control",
            )
            .view()
            .await?
            .json::<OperationControls>()
    }
}
