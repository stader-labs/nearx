use crate::constants::gas::*;
use crate::constants::MIN_BALANCE_FOR_STORAGE;
use crate::errors::*;
use crate::events::*;
use crate::utils::*;
use crate::{
    constants::{gas, NO_DEPOSIT},
    contract::*,
    errors::ERROR_VALIDATOR_IS_BUSY,
    state::*,
};
use near_sdk::{env, log, near_bindgen, require, ONE_NEAR};

#[near_bindgen]
impl NearxPool {
    // keep calling this method until false is return
    pub fn epoch_stake(&mut self) -> bool {
        self.assert_epoch_stake_not_paused();

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

        let validator = self.get_validator_to_stake();
        require!(validator.is_some(), ERROR_NO_VALIDATOR_AVAILABLE_TO_STAKE);

        let validator = validator.unwrap();

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

    #[private]
    pub fn on_stake_pool_deposit_and_stake(&mut self, validator: AccountId, amount: Balance) {
        let mut validator_info = self.internal_get_validator(&validator);
        if is_promise_success() {
            validator_info.staked += amount;

            Event::EpochStakeCallbackSuccess {
                validator_id: validator.clone(),
                amount: U128(amount),
            }
            .emit();
        } else {
            self.reconciled_epoch_stake_amount += amount;

            Event::EpochStakeCallbackFailed {
                validator_id: validator.clone(),
                amount: U128(amount),
            }
            .emit();
        }

        self.internal_update_validator(&validator, &validator_info);
    }

    pub fn epoch_autocompound_rewards(&mut self, validator: AccountId) {
        self.assert_epoch_autocompounding_not_paused();

        let min_gas = AUTOCOMPOUND_EPOCH
            + ON_STAKE_POOL_GET_ACCOUNT_STAKED_BALANCE
            + ON_STAKE_POOL_GET_ACCOUNT_STAKED_BALANCE_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        let validator_info = self.internal_get_validator(&validator);

        require!(!validator_info.paused(), ERROR_VALIDATOR_IS_BUSY);

        let epoch_height = env::epoch_height();

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
        validator_info.last_redeemed_rewards_epoch = env::epoch_height();

        //new_total_balance has the new staked amount for this pool
        let new_total_balance = total_staked_balance.0;
        log!("total staked balance is {}", total_staked_balance.0);

        //compute rewards, as new balance minus old balance
        let rewards = new_total_balance.saturating_sub(validator_info.staked);

        log!(
            "validator account:{} old_balance:{} new_balance:{} rewards:{}",
            validator_info.account_id,
            validator_info.total_balance(),
            new_total_balance,
            rewards
        );

        self.internal_update_validator(&validator_info.account_id, &validator_info);

        if rewards > 0 {
            //updated accumulated_staked_rewards value for the contract
            self.accumulated_staked_rewards += rewards;
            //updated new "staked" value for this pool
            validator_info.staked = new_total_balance;

            let operator_fee = rewards * self.rewards_fee;
            log!(format!("operator_fee is {:?}", operator_fee));
            self.total_staked += rewards;
            let treasury_account_shares =
                self.num_shares_from_staked_amount_rounded_down(operator_fee);
            log!(format!("total_staked is {:?}", self.total_staked));
            log!(format!("total shares is {:?}", self.total_stake_shares));

            self.internal_update_validator(&validator_info.account_id, &validator_info);

            if treasury_account_shares > 0 {
                // Mint shares for the treasury account
                let treasury_account_id = self.treasury_account_id.clone();
                let mut treasury_account = self.internal_get_account(&treasury_account_id);
                treasury_account.stake_shares += treasury_account_shares;
                self.total_stake_shares += treasury_account_shares;
                self.internal_update_account(&treasury_account_id, &treasury_account);

                PromiseOrValue::Value(true)
            } else {
                PromiseOrValue::Value(false)
            }
        } else {
            PromiseOrValue::Value(false)
        }
    }

    pub fn epoch_unstake(&mut self) -> bool {
        self.assert_epoch_unstake_not_paused();

        let min_gas = UNSTAKE_EPOCH + ON_STAKE_POOL_UNSTAKE + ON_STAKE_POOL_UNSTAKE_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        self.epoch_reconcilation();

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

    #[private]
    pub fn on_stake_pool_unstake(&mut self, validator_id: AccountId, amount_to_unstake: u128) {
        let mut validator = self.internal_get_validator(&validator_id);

        if is_promise_success() {
            validator.unstaked_amount += amount_to_unstake;

            Event::EpochUnstakeCallbackSuccess {
                validator_id: validator_id.clone(),
                amount: U128(amount_to_unstake),
            }
            .emit();
        } else {
            self.reconciled_epoch_unstake_amount += amount_to_unstake;
            validator.staked += amount_to_unstake;
            validator.unstake_start_epoch = validator.last_unstake_start_epoch;

            Event::EpochUnstakeCallbackFailed {
                validator_id: validator_id.clone(),
                amount: U128(amount_to_unstake),
            }
            .emit();
        }

        self.internal_update_validator(&validator_id, &validator);
    }

    pub fn epoch_withdraw(&mut self, validator: AccountId) {
        self.assert_epoch_withdraw_not_paused();

        // make sure enough gas was given
        let min_gas = WITHDRAW_EPOCH + ON_STAKE_POOL_WITHDRAW_ALL + ON_STAKE_POOL_WITHDRAW_ALL_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

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

        validator_info.unstaked_amount -= amount;

        self.internal_update_validator(&validator, &validator_info);

        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_static_gas(gas::ON_STAKE_POOL_WITHDRAW_ALL)
            .with_attached_deposit(NO_DEPOSIT)
            .withdraw(U128(amount))
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_WITHDRAW_ALL_CB)
                    .on_stake_pool_withdraw_all(validator_info, amount),
            );

        Event::EpochWithdrawAttempt {
            validator_id: validator,
            amount: U128(amount),
        }
        .emit();
    }

    #[private]
    pub fn on_stake_pool_withdraw_all(&mut self, validator_info: ValidatorInfo, amount: u128) {
        if !is_promise_success() {
            let mut validator_info = self.internal_get_validator(&validator_info.account_id);
            validator_info.unstaked_amount += amount;
            self.internal_update_validator(&validator_info.account_id, &validator_info);

            Event::EpochWithdrawCallbackSuccess {
                validator_id: validator_info.account_id,
                amount: U128(amount),
            }
            .emit();
        } else {
            Event::EpochWithdrawCallbackFailed {
                validator_id: validator_info.account_id,
                amount: U128(amount),
            }
            .emit();
        }
    }

    pub fn sync_balance_from_validator(&mut self, validator_id: AccountId) {
        self.assert_sync_validator_balance_not_paused();

        let min_gas = SYNC_VALIDATOR_EPOCH
            + ON_STAKE_POOL_GET_ACCOUNT_TOTAL_BALANCE
            + ON_STAKE_POOL_GET_ACCOUNT_TOTAL_BALANCE_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        let validator_info = self.internal_get_validator(&validator_id);

        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_static_gas(gas::ON_STAKE_POOL_GET_ACCOUNT_TOTAL_BALANCE)
            .with_attached_deposit(NO_DEPOSIT)
            .get_account(env::current_account_id())
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_GET_ACCOUNT_TOTAL_BALANCE_CB)
                    .on_stake_pool_get_account(validator_info.account_id),
            );
    }

    #[private]
    pub fn on_stake_pool_get_account(
        &mut self,
        validator_id: AccountId,
        #[callback] account: HumanReadableAccount,
    ) {
        let mut validator = self.internal_get_validator(&validator_id);

        let new_total_balance = account.staked_balance.0 + account.unstaked_balance.0;
        require!(
            abs_diff_eq(new_total_balance, validator.total_balance(), 1),
            ERROR_VALIDATOR_TOTAL_BALANCE_OUT_OF_SYNC
        );

        require!(
            abs_diff_eq(account.staked_balance.0, validator.staked, 200),
            ERROR_VALIDATOR_STAKED_BALANCE_OUT_OF_SYNC
        );
        require!(
            abs_diff_eq(account.unstaked_balance.0, validator.unstaked_amount, 200),
            ERROR_VALIDATOR_UNSTAKED_BALANCE_OUT_OF_SYNC
        );

        // update balance
        validator.staked = account.staked_balance.0;
        validator.unstaked_amount = account.unstaked_balance.0;

        self.internal_update_validator(&validator_id, &validator);

        Event::BalanceSyncedFromValidator {
            validator_id,
            staked_balance: account.staked_balance,
            unstaked_balance: account.unstaked_balance,
        }
        .emit();
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

    pub fn drain_unstake(&mut self, validator: AccountId) {
        self.assert_operator_or_owner();

        let min_gas = DRAIN_UNSTAKE + ON_STAKE_POOL_UNSTAKE + ON_STAKE_POOL_UNSTAKE_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        let mut validator_info = self.internal_get_validator(&validator);

        // make sure the validator:
        // 1. has been paused
        // 2. not in pending release
        // 3. has not unstaked balance (because this part is from user's unstake request)
        require!(validator_info.paused(), ERROR_VALIDATOR_NOT_PAUSED);
        require!(
            !validator_info.pending_unstake_release(),
            ERROR_VALIDATOR_UNSTAKE_STILL_UNBONDING
        );
        require!(
            validator_info.unstaked_amount == 0,
            ERROR_NON_POSITIVE_UNSTAKE_AMOUNT
        );

        let amount_to_unstake = validator_info.staked;

        validator_info.staked -= amount_to_unstake;
        validator_info.last_unstake_start_epoch = validator_info.unstake_start_epoch;
        validator_info.unstake_start_epoch = env::epoch_height();

        self.internal_update_validator(&validator, &validator_info);

        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_static_gas(gas::ON_STAKE_POOL_UNSTAKE)
            .with_attached_deposit(NO_DEPOSIT)
            .unstake(U128(amount_to_unstake))
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_UNSTAKE_CB)
                    .on_stake_pool_drain_unstake(
                        validator_info.account_id.clone(),
                        amount_to_unstake,
                    ),
            );
        Event::DrainUnstake {
            account_id: validator,
            amount: U128(amount_to_unstake),
        }
        .emit();
    }

    #[private]
    pub fn on_stake_pool_drain_unstake(
        &mut self,
        validator_id: AccountId,
        amount_to_unstake: u128,
    ) {
        let mut validator = self.internal_get_validator(&validator_id);

        if is_promise_success() {
            validator.unstaked_amount += amount_to_unstake;

            Event::EpochUnstakeCallbackSuccess {
                validator_id: validator_id.clone(),
                amount: U128(amount_to_unstake),
            }
            .emit();
        } else {
            validator.staked += amount_to_unstake;
            validator.unstake_start_epoch = validator.last_unstake_start_epoch;

            Event::EpochUnstakeCallbackFailed {
                validator_id: validator_id.clone(),
                amount: U128(amount_to_unstake),
            }
            .emit();
        }

        self.internal_update_validator(&validator_id, &validator);
    }

    /// Withdraw from a drained validator
    pub fn drain_withdraw(&mut self, validator: AccountId) {
        self.assert_operator_or_owner();

        // make sure enough gas was given
        let min_gas = DRAIN_WITHDRAW + ON_STAKE_POOL_WITHDRAW_ALL + ON_STAKE_POOL_WITHDRAW_ALL_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        let mut validator_info = self.internal_get_validator(&validator);

        // make sure the validator:
        // 1. has weight set to 0
        // 2. has no staked balance
        // 3. not pending release
        require!(validator_info.paused(), ERROR_VALIDATOR_NOT_PAUSED);
        require!(validator_info.staked == 0, ERROR_NON_POSITIVE_STAKE_AMOUNT);
        require!(
            !validator_info.pending_unstake_release(),
            ERROR_VALIDATOR_UNSTAKE_STILL_UNBONDING
        );

        let amount = validator_info.unstaked_amount;
        validator_info.unstaked_amount -= amount;

        self.internal_update_validator(&validator, &validator_info);

        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_static_gas(gas::ON_STAKE_POOL_WITHDRAW_ALL)
            .with_attached_deposit(NO_DEPOSIT)
            .withdraw(U128(amount))
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_WITHDRAW_ALL_CB)
                    .on_stake_pool_drain_withdraw(validator_info.account_id, amount),
            );
        Event::DrainWithdraw {
            account_id: validator,
            amount: U128(amount),
        }
        .emit();
    }

    #[private]
    pub fn on_stake_pool_drain_withdraw(
        &mut self,
        validator_id: AccountId,
        amount_to_withdraw: u128,
    ) {
        let mut validator_info = self.internal_get_validator(&validator_id);

        if is_promise_success() {
            // stake the drained amount into the next epoch
            self.user_amount_to_stake_in_epoch += amount_to_withdraw;
        } else {
            validator_info.unstaked_amount += amount_to_withdraw;
            self.internal_update_validator(&validator_id, &validator_info);
        }
    }
}
