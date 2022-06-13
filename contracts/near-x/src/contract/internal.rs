use crate::errors::*;
use crate::{
    constants::{
        gas, MIN_BALANCE_FOR_STORAGE, MIN_UNSTAKE_AMOUNT, NO_DEPOSIT, UNSTAKE_COOLDOWN_EPOCH,
    },
    contract::*,
    errors,
    state::*,
};
use near_sdk::{log, require, AccountId, Balance, EpochHeight, Promise, PromiseOrValue, ONE_NEAR};

#[near_bindgen]
impl NearxPool {
    pub(crate) fn internal_deposit_and_stake(&mut self, amount: u128) {
        self.assert_staking_not_paused();

        self.assert_min_deposit_amount(amount);

        let account_id = env::predecessor_account_id();
        let mut account = self.internal_get_account(&account_id);

        // Calculate the number of "stake" shares that the account will receive for staking the
        // given amount.
        let num_shares = self.stake_shares_from_amount(amount);
        require!(num_shares > 0, ERROR_NON_POSITIVE_STAKE_SHARES);

        account.stake_shares += num_shares;
        self.internal_update_account(&account_id, &account);

        self.total_staked += amount;
        self.total_stake_shares += num_shares;

        // Increase requested stake amount within the current epoch
        self.user_amount_to_stake_unstake += Direction::Stake(amount);

        log!(
            "Total NEAR staked is {}. Total NEARX supply is {}",
            self.total_staked,
            self.total_stake_shares
        );
    }

    /// mints NearX based on user's deposited amount and current NearX price
    pub(crate) fn internal_deposit_and_stake_direct_stake(&mut self, user_amount: Balance) {
        log!("User deposited amount is {}", user_amount);
        self.assert_not_busy();

        self.assert_min_deposit_amount(user_amount);

        self.assert_staking_not_paused();

        let account_id = env::predecessor_account_id();

        // Calculate the number of nearx (stake shares) that the account will receive for staking the given amount.
        let num_shares = self.stake_shares_from_amount(user_amount);
        require!(num_shares > 0);

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
                    .with_static_gas(gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE)
                    .on_stake_pool_deposit_and_stake_direct(
                        selected_validator,
                        user_amount,
                        num_shares,
                        account_id,
                    ),
            );
    }

    pub(crate) fn internal_unstake(&mut self, near_amount: Option<Balance>) {
        let account_id = env::predecessor_account_id();
        let mut account = self.internal_get_account(&account_id);
        let (near_amount, nearx_amount) = match near_amount {
            Some(amount) => (amount, self.stake_shares_from_amount(amount)),
            None => (
                self.amount_from_stake_shares(account.stake_shares),
                account.stake_shares,
            ),
        };

        log!("User unstaked amount is {}", near_amount);

        require!(nearx_amount != 0, errors::UNSTAKE_AMOUNT_ZERO);
        require!(
            account.stake_shares >= nearx_amount,
            errors::NOT_ENOUGH_SHARES
        );

        // User account update:
        account.stake_shares -= nearx_amount;
        account.unstaked += near_amount;
        // Reset the withdraw cooldown:
        account.withdrawable_epoch =
            env::epoch_height() + self.num_epoch_to_unstake(account.unstaked);
        self.internal_update_account(&account_id, &account);

        // Pool update:
        self.user_amount_to_stake_unstake += Direction::Unstake(near_amount);
        self.total_stake_shares -= nearx_amount;
        self.total_staked -= near_amount;

        log!("Successfully unstaked {}", near_amount);
    }

    pub(crate) fn internal_withdraw(&mut self, near_amount: Option<Balance>) {
        let account_id = env::predecessor_account_id();
        let mut account = self.internal_get_account(&account_id);
        let near_amount = near_amount.unwrap_or(account.unstaked);

        log!("User withdrawn amount is {}", near_amount);

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

        account.unstaked -= near_amount;
        self.internal_update_account(&account_id, &account);

        Promise::new(account_id).transfer(near_amount);
    }
}

// Epoch stuff:
#[near_bindgen]
impl NearxPool {
    pub(crate) fn internal_epoch_stake(&mut self) -> bool {
        // make sure enough gas was given
        // TODO - bchain - scope the gas into a module to make these constants more readable
        let min_gas = gas::STAKE_EPOCH
            + gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE
            + gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        let amount_to_stake = match self.stake_unstake_locked_in_epoch {
            Direction::Stake(stake) => stake,
            Direction::Unstake(_) => panic!("Invalid state"),
        };

        // after cleanup, there might be no need to stake
        if amount_to_stake == 0 {
            log!("no need to stake, amount to settle is zero");
            return false;
        }

        // TODO - bchain we might have to change the validator staking logic
        let validator = self
            .get_validator_with_min_stake()
            .expect(ERROR_NO_VALIDATOR_AVAILABLE_TO_STAKE);

        if amount_to_stake < ONE_NEAR {
            log!("stake amount too low: {}", amount_to_stake);
            return false;
        }

        require!(
            env::account_balance() >= amount_to_stake + MIN_BALANCE_FOR_STORAGE,
            ERROR_MIN_BALANCE_FOR_CONTRACT_STORAGE
        );

        // update internal state
        self.stake_unstake_locked_in_epoch = Direction::Stake(0);

        // do staking on selected validator
        ext_staking_pool::ext(validator.account_id.clone())
            .with_attached_deposit(amount_to_stake)
            .with_static_gas(gas::DEPOSIT_AND_STAKE)
            .deposit_and_stake()
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE)
                    .on_stake_pool_deposit_and_stake(validator.account_id, amount_to_stake),
            );

        true
    }

    pub(crate) fn internal_epoch_unstake(&mut self) -> PromiseOrValue<bool> {
        let amount_to_unstake = match self.stake_unstake_locked_in_epoch {
            Direction::Stake(_) => panic!("Invalid state"),
            Direction::Unstake(amount) => amount,
        };

        if amount_to_unstake == 0 {
            log!("Nothing to unstake");
            PromiseOrValue::Value(false)
        } else if amount_to_unstake < MIN_UNSTAKE_AMOUNT {
            // We will not lock a validator only to unstake a small amount:
            log!("Not enough to unstake");
            PromiseOrValue::Value(false)
        } else if let Some(mut validator_info) = self.validator_available_for_unstake() {
            let to_unstake = amount_to_unstake;

            // Validator update:
            validator_info.staked -= to_unstake;
            // Pool update:
            self.stake_unstake_locked_in_epoch.decrease(to_unstake);
            self.to_withdraw += to_unstake;
            ext_staking_pool::ext(validator_info.account_id.clone())
                .with_static_gas(gas::DEPOSIT_AND_STAKE)
                .unstake(to_unstake.into())
                .then(
                    ext_staking_pool_callback::ext(env::current_account_id())
                        .with_static_gas(gas::ON_STAKING_POOL_UNSTAKE)
                        .on_stake_pool_epoch_unstake(validator_info, to_unstake),
                )
                .into()
        } else {
            log!("No suitable validator found to unstake from");
            PromiseOrValue::Value(false)
        }
    }

    pub(crate) fn internal_epoch_withdraw(
        &mut self,
        account_id: AccountId,
    ) -> PromiseOrValue<bool> {
        let validator_info = self.internal_get_validator(&account_id);
        self.internal_update_validator(&ValidatorInfo {
            unstaked: 0,
            ..validator_info.clone()
        });

        ext_staking_pool::ext(account_id)
            .with_static_gas(gas::DEPOSIT_AND_STAKE)
            .withdraw_all()
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_static_gas(gas::ON_STAKING_POOL_UNSTAKE)
                    .on_stake_pool_epoch_withdraw(validator_info),
            )
            .into()
    }

    /// Reconcile the amounts to stake and unstake in this epoch.
    /// After the reconciliation, one of those amounts is set to zero.
    pub(crate) fn internal_epoch_lock_stake_unstake(&mut self) {
        let current_epoch = env::epoch_height();

        if current_epoch != self.last_reconcilation_epoch {
            // First time we run the lock this epoch:

            self.stake_unstake_locked_in_epoch += self.user_amount_to_stake_unstake;
            self.user_amount_to_stake_unstake = Direction::Stake(0);

            self.last_reconcilation_epoch = current_epoch;
        }
    }
}

// Data manipulation
#[near_bindgen] //TODO: remove the attribute?
impl NearxPool {
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

    pub fn get_validator_with_min_stake(&self) -> Option<ValidatorInfo> {
        self.validator_info_map
            .values()
            .filter(|v| v.unlocked())
            .min_by_key(|v| v.staked)
    }

    pub(crate) fn num_epoch_to_unstake(&self, amount: u128) -> EpochHeight {
        let mut available_amount: Balance = 0;
        let mut total_staked_amount: Balance = 0;
        for validator in self.validator_info_map.values() {
            total_staked_amount += validator.staked;

            if validator.available() == false && validator.staked > 0 {
                available_amount += validator.staked;
            }

            // found enough balance to unstake from available validators
            if available_amount >= amount {
                return UNSTAKE_COOLDOWN_EPOCH;
            }
        }

        if total_staked_amount == 0 {
            // nothing is actually staked, all balance should be available now
            // still leave a buffer for the user
            UNSTAKE_COOLDOWN_EPOCH
        } else {
            // no enough available validators to unstake
            // double the unstake waiting time
            2 * UNSTAKE_COOLDOWN_EPOCH
        }
    }
}
