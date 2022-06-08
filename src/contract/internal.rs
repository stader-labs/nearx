use crate::{
    constants::{gas, MIN_BALANCE_FOR_STORAGE, MIN_STAKE_AMOUNT, MIN_UNSTAKE_AMOUNT, NO_DEPOSIT},
    contract::*,
    errors,
    state::*,
    utils::{assert_callback_calling, fallible_subassign},
};
use near_sdk::{
    is_promise_success, log, require, AccountId, Balance, Promise, PromiseOrValue, ONE_NEAR,
};

#[near_bindgen]
impl NearxPool {
    /// mints NearX based on user's deposited amount and current NearX price
    pub(crate) fn internal_deposit_and_stake(&mut self, user_amount: Balance) {
        log!("User deposited amount is {}", user_amount);
        self.assert_not_busy();

        self.assert_min_deposit_amount(user_amount);

        self.assert_staking_not_paused();

        let account_id = env::predecessor_account_id();

        // Calculate the number of nearx (stake shares) that the account will receive for staking the given amount.
        let num_shares = self.stake_shares_from_amount(user_amount);
        assert!(num_shares > 0);

        let selected_validator = self
            .validator_with_min_stake()
            .expect(errors::VALIDATORS_ARE_BUSY);

        log!("Amount is {}", user_amount);
        //schedule async deposit_and_stake on that pool
        ext_staking_pool::ext(selected_validator.account_id.clone())
            .with_static_gas(gas::DEPOSIT_AND_STAKE)
            .with_attached_deposit(user_amount)
            .deposit_and_stake()
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKING_POOL_DEPOSIT_AND_STAKE)
                    .on_stake_pool_deposit_and_stake(
                        selected_validator,
                        user_amount,
                        num_shares,
                        account_id,
                    ),
            );
    }

    pub fn on_stake_pool_deposit_and_stake(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
        amount: u128,
        shares: u128,
        user: AccountId,
    ) -> PromiseOrValue<bool> {
        assert_callback_calling();

        let mut acc = &mut self.accounts.get(&user).unwrap_or_default();
        let mut transfer_funds = false;

        let stake_succeeded = is_promise_success();
        println!("stake_succeeded {:?}", stake_succeeded);

        if stake_succeeded {
            validator_info.staked += amount;
            acc.stake_shares += shares;
            self.total_stake_shares += shares;
            self.total_staked += amount;
            log!(
                "Successfully staked {} into {}",
                amount,
                validator_info.account_id
            );
        } else {
            log!(
                "Failed to stake {} into {}",
                amount,
                validator_info.account_id
            );
            transfer_funds = true;
            validator_info.lock = false;
            self.contract_lock = false;
        }

        self.internal_update_validator(&validator_info);

        if transfer_funds {
            log!("Transfering back {} to {} after stake failed", amount, user);
            PromiseOrValue::Promise(Promise::new(user).transfer(amount))
        } else {
            log!("Reconciling total staked balance");
            self.internal_update_account(&user, acc);
            // Reconcile the total staked amount to the right value
            ext_staking_pool::ext(validator_info.account_id.clone())
                .with_static_gas(gas::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE)
                .with_attached_deposit(NO_DEPOSIT)
                .get_account_staked_balance(env::current_account_id())
                .then(
                    ext_staking_pool_callback::ext(env::current_account_id())
                        .with_attached_deposit(NO_DEPOSIT)
                        .with_static_gas(gas::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE)
                        .on_get_sp_staked_balance_reconcile(validator_info, amount),
                );
            PromiseOrValue::Value(true)
        }
    }

    pub(crate) fn internal_unstake(&mut self, near_amount: Balance) {
        log!("User unstaked amount is {}", near_amount);
        let account_id = env::predecessor_account_id();
        let mut account = self.internal_get_account(&account_id);
        let nearx_amount = self.stake_shares_from_amount(near_amount);

        require!(nearx_amount != 0, errors::UNSTAKE_AMOUNT_ZERO);
        require!(
            account.stake_shares >= nearx_amount,
            errors::NOT_ENOUGH_SHARES
        );

        // User account update:
        fallible_subassign(&mut account.stake_shares, nearx_amount);
        account.unstaked += near_amount;
        // Pool update:
        self.to_unstake += near_amount;
        fallible_subassign(&mut self.total_stake_shares, nearx_amount);

        self.internal_update_account(&account_id, &account);

        log!("Successfully unstaked {}", near_amount);
    }

    pub(crate) fn internal_epoch_unstake(&mut self) -> PromiseOrValue<bool> {
        if self.to_unstake == 0 {
            log!("Nothing to unstake");
            PromiseOrValue::Value(false)
        } else if self.to_unstake < MIN_UNSTAKE_AMOUNT {
            log!("Not enough to unstake");
            PromiseOrValue::Value(false)
        } else if let Some(mut validator_info) = self.validator_available_for_unstake() {
            let to_unstake = (validator_info.staked - ONE_NEAR).min(self.to_unstake);

            if to_unstake < MIN_UNSTAKE_AMOUNT {
                PromiseOrValue::Value(false)
            } else {
                // Validator update:
                fallible_subassign(&mut validator_info.staked, to_unstake);
                // Pool update:
                fallible_subassign(&mut self.total_staked, to_unstake);
                fallible_subassign(&mut self.to_unstake, to_unstake);
                self.to_withdraw += to_unstake;
                //TODO Not sure if it must be sustracted here, or during withdraw
                ext_staking_pool::ext(validator_info.account_id.clone())
                    .with_static_gas(gas::DEPOSIT_AND_STAKE)
                    .unstake(to_unstake.into())
                    .then(
                        ext_staking_pool_callback::ext(env::current_account_id())
                            .with_static_gas(gas::ON_STAKING_POOL_UNSTAKE)
                            .epoch_unstake_callback(validator_info, to_unstake.into()),
                    )
                    .into()
            }
        } else {
            log!("No suitable validator found to unstake from");
            PromiseOrValue::Value(false)
        }
    }

    #[private]
    #[allow(dead_code)] // The code isn't dead actually
    fn epoch_unstake_callback(
        &mut self,
        mut validator_info: ValidatorInfo,
        near_amount: Balance,
    ) -> PromiseOrValue<bool> {
        assert_callback_calling();

        let unstake_succeeded = is_promise_success();
        println!("unstake_succeeded {:?}", unstake_succeeded);

        if unstake_succeeded {
            log!(
                "Successfully unstaked {} from {}",
                near_amount,
                validator_info.account_id,
            );

            validator_info.to_withdraw += near_amount;
        } else {
            log!(
                "Failed to unstake {} from {}",
                near_amount,
                validator_info.account_id,
            );

            // ROLLBACK:
            // Validator update:
            validator_info.staked += near_amount;
            // Pool update:
            self.total_staked += near_amount;
            self.to_unstake += near_amount;
            self.to_withdraw -= near_amount;

            validator_info.lock = false;
            self.contract_lock = false;
        }

        self.internal_update_validator(&validator_info);

        PromiseOrValue::Value(true)
    }

    pub(crate) fn internal_epoch_withdraw(&mut self) -> PromiseOrValue<bool> {
        match self
            .validator_info_map
            .values()
            .find(|v| v.to_withdraw != 0)
        {
            Some(validator_info) => ext_staking_pool::ext(validator_info.account_id.clone())
                .with_static_gas(gas::DEPOSIT_AND_STAKE)
                .withdraw_all()
                .then(
                    ext_staking_pool_callback::ext(env::current_account_id())
                        .with_static_gas(gas::ON_STAKING_POOL_UNSTAKE)
                        .epoch_withdraw_callback(validator_info),
                )
                .into(),
            None => PromiseOrValue::Value(false),
        }
    }

    #[private]
    #[allow(dead_code)] // The code isn't dead actually
    fn epoch_withdraw_callback(
        &mut self,
        mut validator_info: ValidatorInfo,
    ) -> PromiseOrValue<bool> {
        assert_callback_calling();

        let withdraw_succeeded = is_promise_success();
        println!("withdraw_succeeded {:?}", withdraw_succeeded);

        validator_info.to_withdraw = 0;

        PromiseOrValue::Value(true)
    }

    pub(crate) fn internal_withdraw(&mut self, near_amount: Balance) {
        log!("User withdrawn amount is {}", near_amount);
        let account_id = env::predecessor_account_id();
        let mut account = self.internal_get_account(&account_id);

        require!(
            account.cooldown_finished(),
            errors::TOKENS_ARE_NOT_READY_FOR_WITHDRAWAL,
        );
        require!(
            near_amount <= account.unstaked,
            errors::NOT_ENOUGH_TOKEN_TO_WITHDRAW,
        );
        require!(
            env::account_balance() - near_amount > MIN_BALANCE_FOR_STORAGE,
            errors::NOT_ENOUGH_TOKEN_TO_WITHDRAW,
        );

        fallible_subassign(&mut account.unstaked, near_amount);
        self.internal_update_account(&account_id, &account);

        Promise::new(account_id).transfer(near_amount);
    }

    pub fn on_get_sp_staked_balance_reconcile(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
        amount_actually_staked: u128,
        #[callback] total_staked_balance: U128,
    ) {
        assert_callback_calling();

        self.contract_lock = false;

        log!("Actual staked amount is {}", amount_actually_staked);

        // difference in staked amount and actual staked amount
        let difference_in_amount = validator_info.staked.saturating_sub(total_staked_balance.0);
        // Reconcile the total staked with the actual total staked amount
        self.total_staked -= difference_in_amount;
        log!("Reconciled total staked to {}", self.total_staked);

        // Reconcile the stake pools total staked with the total staked balance
        validator_info.staked = total_staked_balance.0;
        validator_info.lock = false;

        self.internal_update_validator(&validator_info);
    }

    pub(crate) fn stake_shares_from_amount(&self, amount: Balance) -> u128 {
        if self.total_stake_shares == 0 {
            amount
        } else if amount == 0 || self.total_staked == 0 {
            0
        } else {
            crate::utils::proportional(self.total_stake_shares, amount, self.total_staked)
        }
    }

    pub(crate) fn amount_from_stake_shares(&self, num_shares: u128) -> u128 {
        if self.total_stake_shares == 0 || num_shares == 0 {
            0
        } else {
            crate::utils::proportional(num_shares, self.total_staked, self.total_stake_shares)
        }
    }

    pub(crate) fn internal_get_validator(&self, validator: &AccountId) -> ValidatorInfo {
        if let Some(val_info) = self.validator_info_map.get(validator) {
            val_info
        } else {
            panic!("{}", errors::VALIDATOR_IS_NOT_PRESENT);
        }
    }

    pub(crate) fn internal_update_validator(&mut self, validator_info: &ValidatorInfo) {
        if validator_info.is_empty() {
            self.validator_info_map.remove(&validator_info.account_id);
        } else {
            self.validator_info_map
                .insert(&validator_info.account_id, validator_info);
        }
    }

    pub(crate) fn internal_get_account(&self, account_id: &AccountId) -> Account {
        self.accounts.get(account_id).unwrap_or_default()
    }

    pub(crate) fn internal_update_account(&mut self, account_id: &AccountId, account: &Account) {
        if account.is_empty() {
            self.accounts.remove(account_id);
        } else {
            self.accounts.insert(account_id, account); //insert_or_update
        }
    }

    pub fn validator_with_min_stake(&self) -> Option<ValidatorInfo> {
        self.validator_info_map
            .values()
            .filter(|v| v.unlocked())
            .min_by_key(|v| v.staked)
    }

    pub fn validator_available_for_unstake(&self) -> Option<ValidatorInfo> {
        self.validator_info_map
            .values()
            .filter(|v| v.unlocked() && v.available() && v.staked > MIN_STAKE_AMOUNT + ONE_NEAR)
            .max_by_key(|v| v.staked)
    }
}
