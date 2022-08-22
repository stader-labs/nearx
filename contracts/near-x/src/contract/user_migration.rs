use crate::constants::*;
use crate::errors::*;
use crate::events::Event;
use crate::{
    constants::{gas, NO_DEPOSIT},
    contract::*,
    state::*,
};
use near_contract_standards::storage_management::StorageManagement;
use near_sdk::{
    is_promise_success, log, require, AccountId, Balance, Promise, PromiseOrValue, ONE_NEAR,
};

// TODO - bchain - remove these post migration
#[near_bindgen]
impl NearxPool {
    pub fn update_user_state(&mut self, user_info: Vec<AccountUpdateRequest>) {
        self.assert_operator_or_owner();
        for u in user_info {
            self.internal_update_account(
                &u.account_id,
                &Account {
                    stake_shares: u.stake_shares.0,
                    unstaked_amount: u.unstaked_amount.0,
                    withdrawable_epoch_height: u.withdrawable_epoch_height,
                },
            );

            self.total_stake_shares += u.stake_shares.0;
            self.total_staked += u.staked_amount.0;
        }
    }

    pub fn migrate_stake_to_validator(&mut self, validator: AccountId, amount: U128) {
        self.assert_operator_or_owner();

        require!(amount.0 > 0, ERROR_REQUIRE_AMOUNT_GT_0);

        // assert that validator exists
        let validator_info = self.internal_get_validator(&validator);

        //schedule async deposit_and_stake on that pool
        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_static_gas(gas::DEPOSIT_AND_STAKE)
            .with_attached_deposit(amount.0)
            .deposit_and_stake()
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE)
                    .on_stake_pool_deposit_and_stake_stake_migration(validator_info, amount),
            );
    }

    #[private]
    pub fn on_stake_pool_deposit_and_stake_stake_migration(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
        amount: U128,
    ) {
        let mut validator_info = self.internal_get_validator(&validator_info.account_id);

        if is_promise_success() {
            validator_info.staked += amount.0;

            log!(
                "Successfully staked {} into {}",
                amount.0,
                validator_info.account_id
            );
            self.internal_update_validator(&validator_info.account_id, &validator_info);
        } else {
            log!(
                "Failed to stake {} into {}",
                amount.0,
                validator_info.account_id
            );
        }
    }

    pub fn update_contract_state(
        &mut self,
        contract_state_update_request: ContractStateUpdateRequest,
    ) {
        self.assert_operator_or_owner();
        self.total_staked = contract_state_update_request
            .total_staked
            .unwrap_or(U128(self.total_staked))
            .0;
        self.total_stake_shares = contract_state_update_request
            .total_stake_shares
            .unwrap_or(U128(self.total_stake_shares))
            .0;
        self.accumulated_staked_rewards = contract_state_update_request
            .accumulated_staked_rewards
            .unwrap_or(U128(self.accumulated_staked_rewards))
            .0;
    }

    pub fn update_validator_state(
        &mut self,
        validator_state_update_request: ValidatorUpdateRequest,
    ) {
        self.assert_operator_or_owner();

        let mut validator_info =
            self.internal_get_validator(&validator_state_update_request.validator_account_id);
        validator_info.staked = validator_state_update_request
            .staked_amount
            .unwrap_or(U128(validator_info.staked))
            .0;
        validator_info.unstaked_amount = validator_state_update_request
            .unstaked_amount
            .unwrap_or(U128(validator_info.unstaked_amount))
            .0;
        self.internal_update_validator(
            &validator_state_update_request.validator_account_id,
            &validator_info,
        );
    }
}
