mod callbacks;
mod internal;
mod operator;
mod public;
mod storage_management;

use crate::state::*;
use near_sdk::json_types::U128;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env, ext_contract, near_bindgen, AccountId, Balance, PanicOnDefault, Promise, PromiseOrValue,
    PublicKey,
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

    /// The amount of tokens to unstake in epoch_unstake.
    pub to_unstake: u128,

    /// The amount of unstaked tokens that will be withdrawn by users.
    pub to_withdraw: u128,

    pub accumulated_staked_rewards: u128,

    pub user_amount_to_stake_in_epoch: Balance,

    // User account map
    pub accounts: UnorderedMap<AccountId, Account>,

    pub validator_info_map: UnorderedMap<AccountId, ValidatorInfo>,

    /// min amount accepted as deposit or stake
    pub min_deposit_amount: u128,

    pub operator_account_id: AccountId,

    pub rewards_fee: Fraction,
}

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

pub struct ContractSourceMetadata {
    /// Commit hash being used for the currently deployed wasm.
    /// If the contract is not open-sourced, this could also be a numbering system
    /// for internal organization / tracking such as "1.0.0" and "2.1.0".
    pub version: String,
    /// Link to open source code such as a Github repository or a CID to somewhere on IPFS.
    pub link: Option<String>,
}

/// Contract for NEP-330.
#[ext_contract(ext_source_metadata)]
pub trait ExtSourceMetadata {
    fn contract_source_metadata() -> ContractSourceMetadata;
}
