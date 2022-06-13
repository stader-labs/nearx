use crate::constants::gas::*;
use crate::constants::{MIN_BALANCE_FOR_STORAGE, ONE_NEAR};
use crate::errors::*;
use crate::events::*;
use crate::utils::is_promise_success;
use crate::{
    constants::{gas, NO_DEPOSIT},
    contract::*,
    errors::ERROR_VALIDATOR_IS_BUSY,
    state::*,
    utils::assert_callback_calling,
};
use near_sdk::{env, log, near_bindgen, require};

#[near_bindgen]
impl NearxPool {
    // keep calling this method until false is return
    pub fn epoch_stake(&mut self) -> bool {
        // make sure enough gas was given
        // TODO - bchain - scope the gas into a module to make these constants more readable
        let min_gas =
            STAKE_EPOCH + ON_STAKE_POOL_DEPOSIT_AND_STAKE + ON_STAKE_POOL_DEPOSIT_AND_STAKE_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        self.epoch_reconcilation();
        // after cleanup, there might be no need to stake
        if self.reconciled_epoch_stake_amount == 0 {
            log!("no need to stake, amount to settle is zero");
            return false;
        }

        // TODO - bchain we might have to change the validator staking logic
        let validator = self.get_validator_to_stake();
        require!(validator.is_some(), ERROR_NO_VALIDATOR_AVAILABLE_TO_STAKE);

        let mut validator = validator.unwrap();

        let amount_to_stake = self.reconciled_epoch_stake_amount;

        if self.reconciled_epoch_stake_amount < ONE_NEAR {
            log!("stake amount too low: {}", amount_to_stake);
            return false;
        }

        require!(
            env::account_balance() >= amount_to_stake + MIN_BALANCE_FOR_STORAGE,
            ERROR_MIN_BALANCE_FOR_CONTRACT_STORAGE
        );

        // update internal state
        self.reconciled_epoch_stake_amount = self
            .reconciled_epoch_stake_amount
            .saturating_sub(amount_to_stake);

        // do staking on selected validator
        ext_staking_pool::ext(validator.account_id.clone())
            .with_attached_deposit(amount_to_stake)
            .with_static_gas(gas::DEPOSIT_AND_STAKE)
            .deposit_and_stake()
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE)
                    .on_stake_pool_deposit_and_stake(validator.account_id.clone(), amount_to_stake),
            );

        Event::EpochStakeAttempt {
            validator_id: validator.account_id,
            amount: U128(amount_to_stake),
        }
        .emit();

        true
    }

    pub fn on_stake_pool_deposit_and_stake(&mut self, validator: AccountId, amount: Balance) {
        assert_callback_calling();

        let mut validator_info = self.internal_get_validator(&validator);
        if is_promise_success() {
            validator_info.staked += amount;
            // reconcile total staked amount to the actual total staked amount
        } else {
            self.reconciled_epoch_stake_amount += amount;
        }

        self.internal_update_validator(&validator, &validator_info);
    }

    pub fn epoch_autocompound_rewards(&mut self, validator: AccountId) {
        self.assert_not_busy();

        let min_gas = AUTOCOMPOUND_EPOCH
            + ON_STAKE_POOL_GET_ACCOUNT_STAKED_BALANCE
            + ON_STAKE_POOL_GET_ACCOUNT_STAKED_BALANCE_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        let mut validator_info = self.internal_get_validator(&validator);

        require!(!validator_info.lock, ERROR_VALIDATOR_IS_BUSY);

        let epoch_height = env::epoch_height();

        println!("validator staked amount is {:?}", validator_info.staked);
        if validator_info.staked == 0 {
            return;
        }

        if validator_info.last_redeemed_rewards_epoch == epoch_height {
            return;
        }

        log!(
            "Fetching total balance from the staking pool {}",
            validator_info.account_id
        );

        self.contract_lock = true;
        validator_info.lock = true;

        self.internal_update_validator(&validator_info.account_id, &validator_info);

        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_attached_deposit(NO_DEPOSIT)
            .with_static_gas(ON_STAKE_POOL_GET_ACCOUNT_STAKED_BALANCE)
            .get_account_staked_balance(env::current_account_id())
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(ON_STAKE_POOL_GET_ACCOUNT_STAKED_BALANCE_CB)
                    .on_get_sp_staked_balance_for_rewards(validator_info),
            );
    }

    #[private]
    pub fn on_get_sp_staked_balance_for_rewards(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
        #[callback] total_staked_balance: U128,
    ) -> PromiseOrValue<bool> {
        assert_callback_calling();

        validator_info.lock = false;
        self.contract_lock = false;

        validator_info.last_redeemed_rewards_epoch = env::epoch_height();

        //new_total_balance has the new staked amount for this pool
        let new_total_balance = total_staked_balance.0;
        log!("total staked balance is {}", total_staked_balance.0);

        //compute rewards, as new balance minus old balance
        let rewards = new_total_balance.saturating_sub(validator_info.total_balance());

        log!(
            "validator account:{} old_balance:{} new_balance:{} rewards:{}",
            validator_info.account_id,
            validator_info.total_balance(),
            new_total_balance,
            rewards
        );

        //updated accumulated_staked_rewards value for the contract
        self.accumulated_staked_rewards += rewards;
        //updated new "staked" value for this pool
        validator_info.staked = new_total_balance;

        let operator_fee = rewards * self.rewards_fee;
        self.total_staked += rewards;

        self.internal_update_validator(&validator_info.account_id, &validator_info);

        if operator_fee > 0 {
            PromiseOrValue::Promise(
                Promise::new(self.operator_account_id.clone()).transfer(operator_fee),
            )
        } else {
            PromiseOrValue::Value(true)
        }
    }

    pub fn epoch_unstake(&mut self) -> bool {
        let min_gas = UNSTAKE_EPOCH + ON_STAKE_POOL_UNSTAKE + ON_STAKE_POOL_UNSTAKE_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        self.epoch_reconcilation();

        println!(
            "reconciled epoch unstake amount is {:?}",
            self.reconciled_epoch_unstake_amount / ONE_NEAR
        );
        // after cleanup, there might be no need to unstake

        if self.reconciled_epoch_unstake_amount == 0 {
            log!("No amount to unstake");
            return false;
        }

        let mut validator = self
            .get_validator_to_unstake()
            .expect(ERROR_NO_VALIDATOR_AVAILABLE_FOR_UNSTAKE);

        let amount_to_unstake =
            std::cmp::min(validator.staked, self.reconciled_epoch_unstake_amount);

        require!(
            amount_to_unstake <= validator.staked,
            ERROR_CANNOT_UNSTAKED_MORE_THAN_STAKED_AMOUNT
        );

        if amount_to_unstake <= ONE_NEAR {
            log!("unstake amount too low: {}", amount_to_unstake);
            return false;
        }

        self.reconciled_epoch_unstake_amount -= amount_to_unstake;
        validator.staked -= amount_to_unstake;
        validator.last_unstake_start_epoch = validator.unstake_start_epoch;
        validator.unstake_start_epoch = env::epoch_height();

        self.internal_update_validator(&validator.account_id, &validator);

        ext_staking_pool::ext(validator.account_id.clone())
            .with_static_gas(gas::ON_STAKE_POOL_UNSTAKE)
            .with_attached_deposit(NO_DEPOSIT)
            .unstake(U128(amount_to_unstake))
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_UNSTAKE_CB)
                    .on_stake_pool_unstake(validator.account_id.clone(), amount_to_unstake),
            );

        Event::EpochUnstakeAttempt {
            validator_id: validator.account_id,
            amount: U128(amount_to_unstake),
        }
        .emit();

        true
    }

    pub fn on_stake_pool_unstake(&mut self, validator_id: AccountId, amount_to_unstake: u128) {
        assert_callback_calling();

        let mut validator = self.internal_get_validator(&validator_id);

        if is_promise_success() {
            validator.unstaked_amount += amount_to_unstake;
        } else {
            self.reconciled_epoch_unstake_amount += amount_to_unstake;
            validator.staked += amount_to_unstake;
            validator.unstake_start_epoch = validator.last_unstake_start_epoch;
        }

        self.internal_update_validator(&validator_id, &validator);
    }

    pub fn epoch_withdraw(&mut self, validator: AccountId) {
        // make sure enough gas was given
        let min_gas = WITHDRAW_EPOCH + ON_STAKE_POOL_WITHDRAW_ALL + ON_STAKE_POOL_WITHDRAW_ALL_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        log!("validator is {:?}", validator);
        let mut validator_info = self.internal_get_validator(&validator);

        require!(
            validator_info.unstaked_amount > 0,
            ERROR_NON_POSITIVE_UNSTAKE_AMOUNT
        );

        require!(
            !validator_info.pending_unstake_release(),
            ERROR_VALIDATOR_UNSTAKE_STILL_UNBONDING
        );

        let amount = validator_info.unstaked_amount;

        validator_info.unstaked_amount = 0;

        self.internal_update_validator(&validator, &validator_info);

        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_static_gas(gas::ON_STAKE_POOL_WITHDRAW_ALL)
            .with_attached_deposit(NO_DEPOSIT)
            .withdraw_all()
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_WITHDRAW_ALL_CB)
                    .on_stake_pool_withdraw_all(validator_info, amount),
            );
    }

    #[private]
    pub fn on_stake_pool_withdraw_all(&mut self, validator_info: ValidatorInfo, amount: u128) {
        assert_callback_calling();
        if !is_promise_success() {
            let mut validator_info =
                self.internal_get_validator(&validator_info.account_id.clone());
            validator_info.unstaked_amount += amount;
            self.internal_update_validator(&validator_info.account_id, &validator_info);
        } else {
            // TODO - emit event
        }
    }

    #[private]
    pub fn epoch_reconcilation(&mut self) {
        if self.last_reconcilation_epoch == env::epoch_height() {
            return;
        }
        self.last_reconcilation_epoch = env::epoch_height();

        // here we use += because cleanup amount might not be 0
        self.reconciled_epoch_stake_amount += self.user_amount_to_stake_in_epoch;
        self.reconciled_epoch_unstake_amount += self.user_amount_to_unstake_in_epoch;
        self.user_amount_to_stake_in_epoch = 0;
        self.user_amount_to_unstake_in_epoch = 0;

        let reconciled_stake_amount = self
            .reconciled_epoch_stake_amount
            .saturating_sub(self.reconciled_epoch_unstake_amount);
        let reconciled_unstake_amount = self
            .reconciled_epoch_unstake_amount
            .saturating_sub(self.reconciled_epoch_stake_amount);

        self.reconciled_epoch_stake_amount = reconciled_stake_amount;
        self.reconciled_epoch_unstake_amount = reconciled_unstake_amount;

        Event::EpochReconcile {
            user_stake_amount: U128(self.reconciled_epoch_stake_amount),
            user_unstake_amount: U128(self.reconciled_epoch_unstake_amount),
        }
        .emit();
    }
}
