use crate::contract::NearxPool;
use crate::errors::*;
use near_sdk::{env, require};

impl NearxPool {
    /// Asserts that the method was called by the owner.
    pub fn assert_owner_calling(&self) {
        require!(
            env::predecessor_account_id() == self.owner_account_id,
            ERROR_UNAUTHORIZED
        )
    }

    pub fn assert_operator_or_owner(&self) {
        require!(
            env::predecessor_account_id() == self.owner_account_id
                || env::predecessor_account_id() == self.operator_account_id,
            ERROR_UNAUTHORIZED
        );
    }

    pub fn assert_min_deposit_amount(&self, amount: u128) {
        require!(amount >= self.min_deposit_amount, ERROR_MIN_DEPOSIT);
    }

    pub fn assert_staking_not_paused(&self) {
        require!(!self.operations_control.stake_paused, ERROR_STAKING_PAUSED);
    }

    pub fn assert_direct_staking_not_paused(&self) {
        require!(
            !self.operations_control.direct_stake_paused,
            ERROR_DIRECT_STAKING_PAUSED
        );
    }

    pub fn assert_unstaking_not_paused(&self) {
        require!(
            !self.operations_control.unstaked_paused,
            ERROR_UNSTAKING_PAUSED
        );
    }

    pub fn assert_withdraw_not_paused(&self) {
        require!(
            !self.operations_control.withdraw_paused,
            ERROR_UNSTAKING_PAUSED
        );
    }

    pub fn assert_staking_epoch_not_paused(&self) {
        require!(
            !self.operations_control.staking_epoch_paused,
            ERROR_STAKING_EPOCH_PAUSED
        );
    }

    pub fn assert_unstaking_epoch_not_paused(&self) {
        require!(
            !self.operations_control.unstaking_epoch_paused,
            ERROR_UNSTAKING_EPOCH_PAUSED
        );
    }

    pub fn assert_epoch_withdraw_not_paused(&self) {
        require!(
            !self.operations_control.withdraw_epoch_paused,
            ERROR_WITHDRAW_EPOCH_PAUSED
        );
    }

    pub fn assert_autocompounding_epoch_not_paused(&self) {
        require!(
            !self.operations_control.autocompounding_epoch_paused,
            ERROR_AUTOCOMPOUNDING_EPOCH_PAUSED
        );
    }

    pub fn assert_sync_validator_balance_not_paused(&self) {
        require!(
            !self.operations_control.sync_validator_balance_paused,
            ERROR_SYNC_VALIDATOR_BALANCE_PAUSED
        );
    }

    pub fn assert_ft_transfer_not_paused(&self) {
        require!(
            !self.operations_control.ft_transfer_paused,
            ERROR_FT_TRANSFER_PAUSED
        );
    }

    pub fn assert_ft_transfer_call_not_paused(&self) {
        require!(
            !self.operations_control.ft_transfer_call_paused,
            ERROR_FT_TRANSFER_CALL_PAUSED
        );
    }
}
