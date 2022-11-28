use crate::errors::*;
use crate::events::*;
use crate::utils::*;
use crate::{
    constants::{gas, NO_DEPOSIT},
    contract::*,
    state::*,
};
use near_sdk::{env, log, near_bindgen, require};

#[near_bindgen]
impl NearxPool {
    // keep calling this method until false is return
    pub fn staking_epoch(&mut self) -> bool {
        self.assert_staking_epoch_not_paused();

        let min_gas = gas::STAKING_EPOCH
            + gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE
            + gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE_CB;
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

        let validator_to_stake_info =
            self.get_validator_to_stake(self.reconciled_epoch_stake_amount);
        require!(
            validator_to_stake_info.0.is_some(),
            ERROR_NO_VALIDATOR_AVAILABLE_TO_STAKE
        );

        let validator = validator_to_stake_info.0.unwrap();

        let amount_to_stake = validator_to_stake_info.1;

        log!("amount to stake is {:?}", amount_to_stake);

        require!(
            env::account_balance() >= amount_to_stake + self.min_storage_reserve,
            ERROR_MIN_BALANCE_FOR_CONTRACT_STORAGE
        );

        // update internal state
        self.reconciled_epoch_stake_amount = self
            .reconciled_epoch_stake_amount
            .checked_sub(amount_to_stake)
            .unwrap();

        // do staking on selected validator
        ext_staking_pool::ext(validator.account_id.clone())
            .with_attached_deposit(amount_to_stake)
            .with_static_gas(gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE)
            .deposit_and_stake()
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE_CB)
                    .on_stake_pool_deposit_and_stake(validator.account_id.clone(), amount_to_stake),
            );

        Event::StakingEpochAttempt {
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

            // all funds staked to public validators thru epoch staking are unstakable
            // at any time. Only funds staked directly with the validator is not unstakable
            // initially all validators should have a non zero max unstakable limit
            validator_info.max_unstakable_limit =
                Some(validator_info.max_unstakable_limit.unwrap_or(0) + amount);

            Event::StakingEpochCallbackSuccess {
                validator_id: validator.clone(),
                amount: U128(amount),
            }
            .emit();
        } else {
            self.reconciled_epoch_stake_amount += amount;

            Event::StakingEpochCallbackFailed {
                validator_id: validator.clone(),
                amount: U128(amount),
            }
            .emit();
        }

        self.internal_update_validator(&validator, &validator_info);
    }

    pub fn autocompounding_epoch(&mut self, validator: AccountId) {
        self.assert_autocompounding_epoch_not_paused();

        let min_gas = gas::AUTOCOMPOUNDING_EPOCH
            + gas::ON_STAKE_POOL_GET_ACCOUNT_STAKED_BALANCE
            + gas::ON_STAKE_POOL_GET_ACCOUNT_STAKED_BALANCE_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        let validator_info = self.internal_get_validator(&validator);

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

        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_attached_deposit(NO_DEPOSIT)
            .with_static_gas(gas::ON_STAKE_POOL_GET_ACCOUNT_STAKED_BALANCE)
            .get_account_staked_balance(env::current_account_id())
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_GET_ACCOUNT_STAKED_BALANCE_CB)
                    .on_get_sp_staked_balance_for_rewards(validator_info),
            );

        Event::AutocompoundingEpochRewardsAttempt {
            validator_id: validator,
        }
        .emit();
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
            validator_info.staked,
            new_total_balance,
            rewards
        );

        self.internal_update_validator(&validator_info.account_id, &validator_info);

        Event::AutocompoundingEpochRewards {
            validator_id: validator_info.account_id.clone(),
            old_balance: U128(validator_info.staked),
            new_balance: U128(new_total_balance),
            rewards: U128(rewards),
        }
        .emit();

        if rewards > 0 {
            //updated accumulated_staked_rewards value for the contract
            self.accumulated_staked_rewards += rewards;
            //updated new "staked" value for this pool
            validator_info.staked = new_total_balance;
            // consider rewards to unstakable since its excess rewards and rewards get distributed to all users
            validator_info.max_unstakable_limit =
                Some(validator_info.max_unstakable_limit.unwrap_or(0) + rewards);

            let operator_fee = rewards * self.rewards_fee;
            log!("operator fee is {}", operator_fee);
            self.total_staked += rewards;
            let treasury_account_shares =
                self.num_shares_from_staked_amount_rounded_down(operator_fee);

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

    pub fn unstaking_epoch(&mut self) -> bool {
        self.assert_unstaking_epoch_not_paused();

        let min_gas =
            gas::UNSTAKING_EPOCH + gas::ON_STAKE_POOL_UNSTAKE + gas::ON_STAKE_POOL_UNSTAKE_CB;
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

        let (validator_to_unstake, validator_unstakable_amount) = self.get_validator_to_unstake();

        require!(
            validator_to_unstake.is_some(),
            ERROR_NO_VALIDATOR_AVAILABLE_FOR_UNSTAKE
        );

        let mut validator_info = validator_to_unstake.unwrap();

        let amount_to_unstake = std::cmp::min(
            validator_unstakable_amount,
            self.reconciled_epoch_unstake_amount,
        );

        require!(
            amount_to_unstake <= validator_info.staked,
            ERROR_CANNOT_UNSTAKED_MORE_THAN_STAKED_AMOUNT
        );

        self.reconciled_epoch_unstake_amount -= amount_to_unstake;
        validator_info.staked -= amount_to_unstake;
        validator_info.last_unstake_start_epoch = validator_info.unstake_start_epoch;
        validator_info.unstake_start_epoch = env::epoch_height();

        self.internal_update_validator(&validator_info.account_id, &validator_info);

        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_static_gas(gas::ON_STAKE_POOL_UNSTAKE)
            .with_attached_deposit(NO_DEPOSIT)
            .unstake(U128(amount_to_unstake))
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_UNSTAKE_CB)
                    .on_stake_pool_unstake(validator_info.account_id.clone(), amount_to_unstake),
            );

        Event::UnstakingEpochAttempt {
            validator_id: validator_info.account_id,
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

            // we might unstake more then the max_unstakable limit at times. This will happen when the max unstakable limit for the
            // validators has not been correctly updated. Ideally this case should never come
            // but we do not want to risk protocol insolvency for this
            let max_unstakable_limit = validator.max_unstakable_limit.unwrap_or(0);
            validator.max_unstakable_limit =
                Some(max_unstakable_limit.saturating_sub(amount_to_unstake));

            Event::UnstakingEpochCallbackSuccess {
                validator_id: validator_id.clone(),
                amount: U128(amount_to_unstake),
            }
            .emit();
        } else {
            self.reconciled_epoch_unstake_amount += amount_to_unstake;
            validator.staked += amount_to_unstake;
            validator.unstake_start_epoch = validator.last_unstake_start_epoch;

            Event::UnstakingEpochCallbackFailed {
                validator_id: validator_id.clone(),
                amount: U128(amount_to_unstake),
            }
            .emit();
        }

        self.internal_update_validator(&validator_id, &validator);
    }

    pub fn withdraw_epoch(&mut self, validator: AccountId) {
        self.assert_epoch_withdraw_not_paused();

        // make sure enough gas was given
        let min_gas = gas::WITHDRAW_EPOCH
            + gas::ON_STAKE_POOL_WITHDRAW_ALL
            + gas::ON_STAKE_POOL_WITHDRAW_ALL_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        let mut validator_info = self.internal_get_validator(&validator);

        // If we run epoch_withdraw before drain_withdraw for a validator, we will loose the drained funds.
        // So don't run epoch_withdraw for a paused validator
        require!(!validator_info.paused(), ERROR_VALIDATOR_IS_PAUSED);
        require!(validator_info.redelegate_to.is_none());
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

        Event::WithdrawEpochAttempt {
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

            Event::WithdrawEpochCallbackFailed {
                validator_id: validator_info.account_id,
                amount: U128(amount),
            }
            .emit();
        } else {
            Event::WithdrawEpochCallbackSuccess {
                validator_id: validator_info.account_id,
                amount: U128(amount),
            }
            .emit();
        }
    }

    pub fn sync_balance_from_validator(&mut self, validator_id: AccountId) {
        self.assert_sync_validator_balance_not_paused();

        let min_gas = gas::SYNC_VALIDATOR_EPOCH
            + gas::ON_STAKE_POOL_GET_ACCOUNT_TOTAL_BALANCE
            + gas::ON_STAKE_POOL_GET_ACCOUNT_TOTAL_BALANCE_CB;
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

        Event::BalanceSyncedFromValidatorAttempt { validator_id }.emit();
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
            abs_diff_eq(new_total_balance, validator.total_balance(), 10000),
            ERROR_VALIDATOR_TOTAL_BALANCE_OUT_OF_SYNC
        );

        require!(
            abs_diff_eq(account.staked_balance.0, validator.staked, 5000),
            ERROR_VALIDATOR_STAKED_BALANCE_OUT_OF_SYNC
        );
        require!(
            abs_diff_eq(account.unstaked_balance.0, validator.unstaked_amount, 5000),
            ERROR_VALIDATOR_UNSTAKED_BALANCE_OUT_OF_SYNC
        );

        Event::BalanceSyncedFromValidator {
            validator_id: validator_id.clone(),
            old_staked_balance: U128(validator.staked),
            old_unstaked_balance: U128(validator.unstaked_amount),
            staked_balance: account.staked_balance,
            unstaked_balance: account.unstaked_balance,
        }
        .emit();

        // update balance
        validator.staked = account.staked_balance.0;
        validator.unstaked_amount = account.unstaked_balance.0;

        self.internal_update_validator(&validator_id, &validator);
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
        let mut reconciled_unstake_amount = self
            .reconciled_epoch_unstake_amount
            .saturating_sub(self.reconciled_epoch_stake_amount);

        // while unstaking first drain the unstaked from the rewards_buffer and then from the validators
        if reconciled_unstake_amount > self.rewards_buffer {
            reconciled_unstake_amount =
                reconciled_unstake_amount.saturating_sub(self.rewards_buffer);
            self.rewards_buffer = 0;
        } else {
            self.rewards_buffer = self
                .rewards_buffer
                .saturating_sub(reconciled_unstake_amount);
            reconciled_unstake_amount = 0;
        }

        self.reconciled_epoch_stake_amount = reconciled_stake_amount;
        self.reconciled_epoch_unstake_amount = reconciled_unstake_amount;

        Event::EpochReconcile {
            actual_epoch_stake_amount: U128(self.user_amount_to_stake_in_epoch),
            actual_epoch_unstake_amount: U128(self.user_amount_to_unstake_in_epoch),
            reconciled_stake_amount: U128(self.reconciled_epoch_stake_amount),
            reconciled_unstake_amount: U128(self.reconciled_epoch_unstake_amount),
        }
        .emit();
    }

    pub fn drain_unstake(&mut self, validator: AccountId) {
        self.assert_operator_or_owner();

        let min_gas =
            gas::DRAIN_UNSTAKE + gas::ON_STAKE_POOL_UNSTAKE + gas::ON_STAKE_POOL_UNSTAKE_CB;
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
        // we have not unstaked.
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
            validator.max_unstakable_limit = Some(0); // entire amount has been unstaked

            Event::DrainUnstakeCallbackSuccess {
                validator_id: validator_id.clone(),
                amount: U128(amount_to_unstake),
            }
            .emit();
        } else {
            validator.staked += amount_to_unstake;
            validator.unstake_start_epoch = validator.last_unstake_start_epoch;

            Event::DrainUnstakeCallbackFail {
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
        let min_gas = gas::DRAIN_WITHDRAW
            + gas::ON_STAKE_POOL_WITHDRAW_ALL
            + gas::ON_STAKE_POOL_WITHDRAW_ALL_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        let mut validator_info = self.internal_get_validator(&validator);

        // make sure the validator:
        // 1. has weight set to 0
        // 2. has no staked balance
        // 3. not pending release
        // 4. we are not in a redelegation phase
        require!(validator_info.redelegate_to.is_none());
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
            validator_id: validator,
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

            Event::DrainWithdrawCallbackSuccess {
                validator_id,
                amount: U128(amount_to_withdraw),
            }
            .emit();
        } else {
            validator_info.unstaked_amount += amount_to_withdraw;
            self.internal_update_validator(&validator_id, &validator_info);

            Event::DrainWithdrawCallbackFail {
                validator_id,
                amount: U128(amount_to_withdraw),
            }
            .emit();
        }
    }

    pub fn rebalance_unstake(&mut self, from_val: AccountId, to_val: AccountId, amount: U128) {
        self.assert_operator_or_owner();

        let mut from_val_info = self.internal_get_validator(&from_val);

        // validator should not have an existing unstake
        require!(from_val_info.unstaked_amount == 0);
        require!(!from_val_info.pending_unstake_release());
        require!(
            from_val_info
                .max_unstakable_limit
                .unwrap_or(from_val_info.staked)
                >= amount.0
        );
        // complete any previous redelegation
        require!(from_val_info.amount_to_redelegate == 0);
        require!(from_val_info.redelegate_to.is_none());

        from_val_info.redelegate_to = Some(to_val.clone());
        from_val_info.staked -= amount.0;
        from_val_info.last_unstake_start_epoch = from_val_info.unstake_start_epoch;
        from_val_info.unstake_start_epoch = env::epoch_height();

        self.internal_update_validator(&from_val, &from_val_info);

        // TODO - add event
        Event::RebalanceUnstake {
            from_validator: from_val.clone(),
            to_validator: to_val.clone(),
            amount,
        }
        .emit();

        ext_staking_pool::ext(from_val_info.account_id.clone())
            .with_static_gas(gas::ON_STAKE_POOL_UNSTAKE)
            .with_attached_deposit(NO_DEPOSIT)
            .unstake(amount)
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_UNSTAKE_CB)
                    .on_stake_pool_rebalance_unstake(from_val.clone(), amount.0),
            );
    }

    #[private]
    pub fn on_stake_pool_rebalance_unstake(&mut self, validator_id: AccountId, amount: u128) {
        let mut validator = self.internal_get_validator(&validator_id);

        if is_promise_success() {
            validator.unstaked_amount += amount;
            validator.amount_to_redelegate += amount;
            validator.max_unstakable_limit = Some(
                validator
                    .max_unstakable_limit
                    .unwrap_or(0)
                    .saturating_sub(amount),
            );

            Event::RebalanceUnstakeCallbackSuccess {
                from_validator: validator_id.clone(),
                to_validator: validator.redelegate_to.as_ref().unwrap().clone(),
                amount: U128(amount),
            }
            .emit();
        } else {
            validator.staked += amount;
            validator.unstake_start_epoch = validator.last_unstake_start_epoch;
            let validator_to_redelegate_to = validator.redelegate_to.as_ref().unwrap().clone();
            validator.redelegate_to = None;

            Event::RebalanceUnstakeCallbackFail {
                from_validator: validator_id.clone(),
                to_validator: validator_to_redelegate_to.clone(),
                amount: U128(amount),
            }
            .emit();
        }

        self.internal_update_validator(&validator_id, &validator);
    }

    #[payable]
    pub fn rebalance_withdraw(&mut self, validator_id: AccountId) {
        self.assert_operator_or_owner();

        // make sure enough gas was given
        let min_gas = gas::DRAIN_WITHDRAW
            + gas::ON_STAKE_POOL_WITHDRAW_ALL
            + gas::ON_STAKE_POOL_WITHDRAW_ALL_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        let mut validator_info = self.internal_get_validator(&validator_id);

        // make sure the validator:
        // 1. has weight set to 0
        // 2. has no staked balance
        // 3. not pending release
        require!(
            !validator_info.pending_unstake_release(),
            ERROR_VALIDATOR_UNSTAKE_STILL_UNBONDING
        );
        require!(validator_info.redelegate_to.is_some());

        let amount = validator_info.amount_to_redelegate;
        validator_info.unstaked_amount -= amount;

        self.internal_update_validator(&validator_id, &validator_info);

        Event::RebalanceWithdraw {
            from_validator: validator_id,
            to_validator: validator_info.redelegate_to.unwrap(),
            amount: U128(amount),
        }
        .emit();

        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_static_gas(gas::ON_STAKE_POOL_WITHDRAW_ALL)
            .with_attached_deposit(NO_DEPOSIT)
            .withdraw(U128(amount))
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_WITHDRAW_ALL_CB)
                    .on_stake_pool_rebalance_withdraw(validator_info.account_id, amount),
            );
    }

    #[private]
    pub fn on_stake_pool_rebalance_withdraw(&mut self, validator_id: AccountId, amount: u128) {
        let mut validator_info = self.internal_get_validator(&validator_id);

        if is_promise_success() {
            // stake the amount in rebalance_stake
            Event::RebalanceWithdrawCallbackSuccess {
                from_validator: validator_id,
                to_validator: validator_info.redelegate_to.unwrap(),
                amount: U128(amount),
            }
            .emit();
        } else {
            validator_info.unstaked_amount += amount;
            self.internal_update_validator(&validator_id, &validator_info);

            Event::RebalanceWithdrawCallbackFail {
                from_validator: validator_id,
                to_validator: validator_info.redelegate_to.unwrap(),
                amount: U128(amount),
            }
            .emit();
        }
    }

    pub fn rebalance_stake(&mut self, validator_id: AccountId) {
        self.assert_operator_or_owner();

        let validator_info = self.internal_get_validator(&validator_id);

        require!(
            !validator_info.pending_unstake_release(),
            ERROR_VALIDATOR_UNSTAKE_STILL_UNBONDING
        );
        // ensure that there is no amount in unbonding period and the amount is withdrawn
        require!(validator_info.unstaked_amount == 0);
        require!(validator_info.redelegate_to.is_some());
        require!(validator_info.amount_to_redelegate > 0);

        let amount_to_stake = validator_info.amount_to_redelegate;
        let validator_to_redelegate_to = validator_info.redelegate_to.unwrap();

        Event::RebalanceStake {
            from_validator: validator_id.clone(),
            to_validator: validator_to_redelegate_to.clone(),
            amount: U128(amount_to_stake),
        }
        .emit();

        ext_staking_pool::ext(validator_id.clone())
            .with_attached_deposit(amount_to_stake)
            .with_static_gas(gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE)
            .deposit_and_stake()
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE_CB)
                    .on_stake_pool_rebalance_deposit_and_stake(
                        validator_id.clone(),
                        validator_to_redelegate_to,
                        amount_to_stake,
                    ),
            );
    }

    #[private]
    pub fn on_stake_pool_rebalance_deposit_and_stake(
        &mut self,
        from_validator_id: AccountId,
        validator_to_redelegate_to: AccountId,
        amount_to_stake: u128,
    ) {
        let mut from_validator_info = self.internal_get_validator(&from_validator_id);
        let mut to_validator_info = self.internal_get_validator(&validator_to_redelegate_to);

        if is_promise_success() {
            // if successful stake
            from_validator_info.redelegate_to = None;
            from_validator_info.amount_to_redelegate = 0;
            to_validator_info.staked += amount_to_stake;

            Event::RebalanceStakeCallbackSuccess {
                from_validator: from_validator_id.clone(),
                to_validator: validator_to_redelegate_to.clone(),
                amount: U128(amount_to_stake),
            }
            .emit();
        } else {
            Event::RebalanceStakeCallbackFail {
                from_validator: from_validator_id.clone(),
                to_validator: validator_to_redelegate_to.clone(),
                amount: U128(amount_to_stake),
            }
            .emit();
        }

        self.internal_update_validator(&from_validator_id, &from_validator_info);
        self.internal_update_validator(&validator_to_redelegate_to, &to_validator_info);
    }
}
