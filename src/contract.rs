mod internal;
mod operator;
mod public;

use crate::state::*;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env, ext_contract, near_bindgen, AccountId, PanicOnDefault, Promise, PromiseOrValue, PublicKey,
};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct NearxPool {
    pub owner_account_id: AccountId,

    pub contract_lock: bool,

    pub staking_paused: bool,

    /// The total amount of tokens actually staked (the tokens are in the staking pools)
    // nearx_price = (total_staked) / (total_stake_shares)
    pub total_staked: u128,

    /// how many "NearX" were minted.
    pub total_stake_shares: u128, //total NearX minted

    pub accumulated_staked_rewards: u128,

    // User account map
    pub accounts: UnorderedMap<AccountId, Account>,

    // list of staking pools
    // TODO - bchain use persistant vector
    pub validators: Vec<ValidatorInfo>,

    /// min amount accepted as deposit or stake
    pub min_deposit_amount: u128,

    pub operator_account_id: AccountId,

    /// operator_rewards_fee_basis_points. (0.2% default) 100 basis point => 1%. E.g.: owner_fee_basis_points=30 => 0.3% owner's fee
    pub rewards_fee_pct: u16,
}

//self-callbacks
#[ext_contract(ext_staking_pool_callback)]
pub trait ExtNearxStakingPoolCallbacks {
    fn on_stake_pool_deposit(&mut self, amount: U128String) -> bool;

    fn on_retrieve_from_staking_pool(&mut self, inx: u16) -> bool;

    fn on_stake_pool_deposit_and_stake(
        &mut self,
        sp_inx: usize,
        amount: u128,
        shares: u128,
        user: AccountId,
    ) -> PromiseOrValue<bool>;

    fn on_get_sp_total_balance(&mut self, sp_inx: usize, #[callback] total_balance: U128String);

    fn on_get_sp_staked_balance_for_rewards(
        &mut self,
        sp_inx: usize,
        #[callback] total_staked_balance: U128String,
    ) -> PromiseOrValue<bool>;

    fn on_get_sp_staked_balance_reconcile(
        &mut self,
        sp_inx: usize,
        amount_actually_staked: u128,
        #[callback] total_staked_balance: U128String,
    );

    fn on_get_sp_unstaked_balance(
        &mut self,
        sp_inx: usize,
        #[callback] unstaked_balance: U128String,
    );
}

#[ext_contract(ext_staking_pool)]
pub trait ExtStakingPool {
    fn get_account_staked_balance(&self, account_id: AccountId) -> U128String;

    fn get_account_unstaked_balance(&self, account_id: AccountId) -> U128String;

    fn get_account_total_balance(&self, account_id: AccountId) -> U128String;

    fn deposit(&mut self);

    fn deposit_and_stake(&mut self);

    fn withdraw(&mut self, amount: U128String);
    fn withdraw_all(&mut self);

    fn stake(&mut self, amount: U128String);

    fn unstake(&mut self, amount: U128String);

    fn unstake_all(&mut self);
}
