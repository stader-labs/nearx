mod callbacks;
mod internal;
mod operator;
mod public;

use crate::state::*;
use near_sdk::json_types::U128;
use near_sdk::{
    collections::UnorderedMap, env, ext_contract, near_bindgen, AccountId, Balance, Promise,
    PromiseOrValue, PublicKey,
};

//self-callbacks
#[ext_contract(ext_staking_pool_callback)]
pub trait ExtNearxStakingPoolCallbacks {
    fn on_stake_pool_deposit(&mut self, amount: U128) -> bool;

    fn on_stake_pool_deposit_and_stake(&mut self, validator: AccountId, amount: Balance);

    fn on_stake_pool_deposit_and_stake_direct(
        &mut self,
        validator_info: ValidatorInfo,
        amount: u128,
        shares: u128,
        user: AccountId,
    ) -> PromiseOrValue<bool>;

    fn on_stake_pool_epoch_unstake(
        &mut self,
        validator_info: ValidatorInfo,
        amount: Balance,
    ) -> PromiseOrValue<bool>;

    fn on_stake_pool_epoch_withdraw(
        &mut self,
        validator_info: ValidatorInfo,
    ) -> PromiseOrValue<bool>;

    fn on_get_sp_total_balance(
        &mut self,
        validator_info: ValidatorInfo,
        #[callback] total_balance: U128,
    );

    fn on_get_sp_staked_balance_for_rewards(
        &mut self,
        validator_info: ValidatorInfo,
        #[callback] total_staked_balance: U128,
    ) -> PromiseOrValue<bool>;

    fn on_get_sp_staked_balance_reconcile(
        &mut self,
        validator_info: ValidatorInfo,
        amount_actually_staked: u128,
        #[callback] total_staked_balance: U128,
    );

    fn on_get_sp_unstaked_balance(
        &mut self,
        validator_info: ValidatorInfo,
        #[callback] unstaked_balance: U128,
    );
}

/// The validators staking pool contract.
#[ext_contract(ext_staking_pool)]
pub trait ExtStakingPool {
    fn get_account_staked_balance(&self, account_id: AccountId) -> U128;
    fn get_account_unstaked_balance(&self, account_id: AccountId) -> U128;
    fn get_account_total_balance(&self, account_id: AccountId) -> U128;

    fn deposit(&mut self);
    fn deposit_and_stake_direct_stake(&mut self);
    fn deposit_and_stake(&mut self);
    fn stake(&mut self, amount: U128);

    fn withdraw(&mut self, amount: U128);
    fn withdraw_all(&mut self);

    fn unstake(&mut self, amount: U128);
    fn unstake_all(&mut self);
}
