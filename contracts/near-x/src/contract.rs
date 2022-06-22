mod internal;
mod metadata;
mod operator;
mod public;

use crate::state::*;
use near_sdk::json_types::U128;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env, ext_contract, near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, Balance, EpochHeight, PanicOnDefault, PromiseOrValue, PublicKey,
};

#[derive(
    Debug, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Copy, PartialEq,
)]
#[serde(crate = "near_sdk::serde")]
pub struct OperationControls {
    pub stake_paused: bool,
    pub unstaked_paused: bool,
    pub withdraw_paused: bool,
    pub epoch_stake_paused: bool,
    pub epoch_unstake_paused: bool,
    pub epoch_withdraw_paused: bool,
    pub epoch_autocompounding_paused: bool,
    pub sync_validator_balance_paused: bool,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct NearxPool {
    pub owner_account_id: AccountId,

    /// The total amount of tokens actually staked (the tokens are in the staking pools)
    // nearx_price = (total_staked) / (total_stake_shares)
    pub total_staked: u128,

    /// how many "NearX" were minted.
    pub total_stake_shares: u128, //total NearX minted

    pub accumulated_staked_rewards: u128,

    /// Amount of NEAR that is users requested to stake
    pub user_amount_to_stake_in_epoch: Balance,
    /// Amount of NEAR that is users requested to unstake
    pub user_amount_to_unstake_in_epoch: Balance,

    /// Amount of NEAR that actually needs to be staked in the epoch
    pub reconciled_epoch_stake_amount: Balance,
    /// Amount of NEAR that actually needs to be unstaked in the epoch
    pub reconciled_epoch_unstake_amount: Balance,
    /// Last epoch height stake/unstake amount were reconciled
    pub last_reconcilation_epoch: EpochHeight,

    // User account map
    pub accounts: UnorderedMap<AccountId, Account>,

    pub validator_info_map: UnorderedMap<AccountId, ValidatorInfo>,

    /// min amount accepted as deposit or stake
    pub min_deposit_amount: u128,

    pub operator_account_id: AccountId,

    pub treasury_account_id: AccountId,

    pub rewards_fee: Fraction,

    // Temp owner for owner update
    // This is to have 2 commit owner update
    pub temp_owner: Option<AccountId>,

    // Operations control
    pub operations_control: OperationControls,
}

//self-callbacks
#[ext_contract(ext_staking_pool_callback)]
pub trait ExtNearxStakingPoolCallbacks {
    fn on_stake_pool_deposit(&mut self, amount: U128) -> bool;

    // TODO - refactor codebase to use accountid instead of validator info
    fn on_stake_pool_deposit_and_stake_direct(
        &mut self,
        validator_info: ValidatorInfo,
        amount: u128,
        shares: u128,
        user: AccountId,
    ) -> PromiseOrValue<bool>;

    fn on_stake_pool_deposit_and_stake_manager(
        &mut self,
        validator_info: ValidatorInfo,
        amount: u128,
        shares: u128,
        user: AccountId,
    ) -> PromiseOrValue<bool>;

    fn on_stake_pool_deposit_and_stake(&mut self, validator: AccountId, amount: u128);

    fn on_stake_pool_withdraw_all(&mut self, validator_info: ValidatorInfo, amount: u128);

    fn on_stake_pool_unstake(&mut self, validator_id: AccountId, amount_to_unstake: u128);

    fn on_stake_pool_drain_unstake(&mut self, validator_id: AccountId, amount_to_unstake: u128);

    fn on_stake_pool_drain_withdraw(&mut self, validator_id: AccountId, amount_to_withdraw: u128);

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

    fn on_stake_pool_get_account(
        &mut self,
        validator_id: AccountId,
        #[callback] account: HumanReadableAccount,
    );
}

#[ext_contract(ext_staking_pool)]
pub trait ExtStakingPool {
    fn get_account_staked_balance(&self, account_id: AccountId) -> U128;

    fn get_account_unstaked_balance(&self, account_id: AccountId) -> U128;

    fn get_account_total_balance(&self, account_id: AccountId) -> U128;

    fn get_account(&self, account_id: AccountId) -> HumanReadableAccount;

    fn deposit(&mut self);

    fn deposit_and_stake(&mut self);

    fn withdraw(&mut self, amount: U128);

    fn withdraw_all(&mut self);

    fn stake(&mut self, amount: U128);

    fn unstake(&mut self, amount: U128);

    fn unstake_all(&mut self);
}
