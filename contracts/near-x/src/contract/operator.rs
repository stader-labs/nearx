use crate::constants::gas::*;
use crate::errors::{self, ERROR_NO_VALIDATOR_AVAILABLE_TO_STAKE};
use crate::{
    constants::{gas, MIN_BALANCE_FOR_STORAGE, NO_DEPOSIT},
    contract::*,
    errors::*,
};
use near_sdk::{log, near_bindgen, require, ONE_NEAR};

#[near_bindgen]
impl NearxPool {
    // keep calling this method until false is return
    pub fn epoch_stake(&mut self) -> bool {
        self.epoch_reconciliation();

        // make sure enough gas was given
        // TODO - bchain - scope the gas into a module to make these constants more readable
        let min_gas =
            STAKE_EPOCH + ON_STAKE_POOL_DEPOSIT_AND_STAKE + ON_STAKE_POOL_DEPOSIT_AND_STAKE_CB;
        require!(
            env::prepaid_gas() >= min_gas,
            format!("{}. require at least {:?}", ERROR_NOT_ENOUGH_GAS, min_gas)
        );

        // after cleanup, there might be no need to stake
        if self.user_amount_to_stake_in_epoch == 0 {
            log!("no need to stake, amount to settle is zero");
            return false;
        }

        // TODO - bchain we might have to change the validator staking logic
        let validator = self
            .get_validator_with_min_stake()
            .expect(ERROR_NO_VALIDATOR_AVAILABLE_TO_STAKE);

        let amount_to_stake = self.user_amount_to_stake_in_epoch;

        if self.user_amount_to_stake_in_epoch < ONE_NEAR {
            log!("stake amount too low: {}", amount_to_stake);
            return false;
        }

        require!(
            env::account_balance() >= amount_to_stake + MIN_BALANCE_FOR_STORAGE,
            ERROR_MIN_BALANCE_FOR_CONTRACT_STORAGE
        );

        // update internal state
        self.user_amount_to_stake_in_epoch = self
            .user_amount_to_stake_in_epoch
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
                    .on_stake_pool_deposit_and_stake(validator.account_id, amount_to_stake),
            );

        true
    }

    pub fn epoch_unstake(&mut self) -> PromiseOrValue<bool> {
        self.epoch_reconciliation();

        self.internal_epoch_unstake()
    }

    /// Reconcile the amounts to stake and unstake in this epoch.
    /// After the reconciliation, one of those amounts is set to zero.
    pub fn epoch_reconciliation(&mut self) {
        let current_epoch = env::epoch_height();
        require!(
            current_epoch != self.last_reconcilation_epoch,
            errors::CANNOT_RECONCILE_TWICE
        );

        if self.user_amount_to_stake_in_epoch >= self.user_amount_to_unstake_in_epoch {
            self.user_amount_to_stake_in_epoch -= self.user_amount_to_unstake_in_epoch;
            self.user_amount_to_unstake_in_epoch = 0;
        } else {
            self.user_amount_to_unstake_in_epoch -= self.user_amount_to_stake_in_epoch;
            self.user_amount_to_stake_in_epoch = 0;
        }

        self.last_reconcilation_epoch = env::epoch_height();
    }

    pub fn epoch_withdraw(&mut self, account_id: AccountId) -> PromiseOrValue<bool> {
        self.internal_epoch_withdraw(account_id)
    }

    pub fn epoch_autocompound_rewards(&mut self, validator: AccountId) {
        self.assert_not_busy();

        let mut validator_info = self.internal_get_validator(&validator);

        assert!(!validator_info.lock, "{}", ERROR_VALIDATOR_IS_BUSY);

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

        self.internal_update_validator(&validator_info);

        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_attached_deposit(NO_DEPOSIT)
            .with_static_gas(gas::GET_ACCOUNT_TOTAL_BALANCE)
            .get_account_staked_balance(env::current_account_id())
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE)
                    .on_get_sp_staked_balance_for_rewards(validator_info),
            );
    }
}
