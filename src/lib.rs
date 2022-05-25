#![allow(clippy::bool_comparison)]

use crate::account::Account;
use crate::constants::{NEAR, ONE_E24};
use crate::errors::{
    ERROR_CONTRACT_BUSY, ERROR_MIN_DEPOSIT, ERROR_STAKING_PAUSED, ERROR_UNAUTHORIZED,
};
use crate::types::{
    HumanReadableAccount, NearxPoolStateResponse, RewardFeeFraction, U128String, U64String,
    ValidatorInfoResponse,
};
use crate::validator::ValidatorInfo;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::Base58PublicKey;
use near_sdk::{env, AccountId, PanicOnDefault, Promise, PromiseOrValue};
use near_sdk::{ext_contract, near_bindgen};

pub mod account;
pub mod constants;
pub mod errors;
pub mod gas;
pub mod internal;
pub mod nearx_token;
pub mod operator;
pub mod types;
pub mod utils;
pub mod validator;

// setup_alloc adds a #[cfg(target_arch = "wasm32")] to the global allocator, which prevents the allocator
// from being used when the contract's main file is used in simulation testing.
near_sdk::setup_alloc!();

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

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct NearxPool {
    pub owner_account_id: AccountId,

    pub contract_lock: bool,

    pub staking_paused: bool,

    /// The total NEAR in the contract
    /// TODO - bchain - We might not need this, we can just use env::account_balance()
    pub contract_account_balance: u128,

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

#[near_bindgen]
impl NearxPool {
    #[init]
    pub fn new(owner_account_id: AccountId, operator_account_id: AccountId) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");

        Self {
            owner_account_id,
            contract_lock: false,
            operator_account_id,
            contract_account_balance: 0,
            staking_paused: false,
            accumulated_staked_rewards: 0,
            total_stake_shares: 0,
            accounts: UnorderedMap::new(b"A".to_vec()),
            min_deposit_amount: NEAR,
            validators: Vec::new(),
            total_staked: 0,
            rewards_fee_pct: 0,
        }
    }

    /*
       Utility stuff
    */
    /// Asserts that the method was called by the owner.
    pub fn assert_owner_calling(&self) {
        assert_eq!(
            &env::predecessor_account_id(),
            &self.owner_account_id,
            "{}",
            ERROR_UNAUTHORIZED
        )
    }
    pub fn assert_operator_or_owner(&self) {
        assert!(
            env::predecessor_account_id() == self.owner_account_id
                || env::predecessor_account_id() == self.operator_account_id,
            "{}",
            ERROR_UNAUTHORIZED
        );
    }

    pub fn assert_not_busy(&self) {
        assert!(!self.contract_lock, "{}", ERROR_CONTRACT_BUSY);
    }

    pub fn assert_min_deposit_amount(&self, amount: u128) {
        assert!(amount >= self.min_deposit_amount, "{}", ERROR_MIN_DEPOSIT);
    }

    pub fn assert_staking_not_paused(&self) {
        assert!(!self.staking_paused, "{}", ERROR_STAKING_PAUSED);
    }

    /*
       Main staking pool api
    */

    /// Rewards claiming
    pub fn ping(&mut self) {}

    #[payable]
    pub fn deposit(&mut self) {
        panic!("Use only deposit_and_stake");
    }

    /// Deposits the attached amount into the inner account of the predecessor and stakes it.
    #[payable]
    pub fn deposit_and_stake(&mut self) {
        self.internal_deposit_and_stake(env::attached_deposit());
    }

    /*
       Staking pool addition and deletion
    */
    pub fn remove_validator(&mut self, inx: u16) {
        self.assert_operator_or_owner();

        let sp = &self.validators[inx as usize];
        if !sp.is_empty() {
            panic!("sp is not empty")
        }
        self.validators.remove(inx as usize);
    }

    pub fn add_validator(&mut self, account_id: AccountId) {
        self.assert_operator_or_owner();
        for inx in 0..self.validators.len() {
            if self.validators[inx].account_id == account_id {
                panic!("already in list");
            }
        }
        self.validators.push(ValidatorInfo::new(account_id));
    }

    pub fn toggle_staking_pause(&mut self) {
        self.assert_operator_or_owner();
        self.staking_paused = !self.staking_paused;
    }

    // View methods

    pub fn get_account_staked_balance(&self, account_id: AccountId) -> U128String {
        self.get_account(account_id).staked_balance
    }

    pub fn get_account_total_balance(&self, account_id: AccountId) -> U128String {
        let acc = self.internal_get_account(&account_id);
        self.amount_from_stake_shares(acc.stake_shares).into()
    }

    pub fn is_account_unstaked_balance_available(&self, account_id: AccountId) -> bool {
        self.get_account(account_id).can_withdraw
    }

    pub fn get_owner_id(&self) -> AccountId {
        self.owner_account_id.clone()
    }

    pub fn get_reward_fee_fraction(&self) -> RewardFeeFraction {
        RewardFeeFraction {
            numerator: self.rewards_fee_pct.into(),
            denominator: 100,
        }
    }

    pub fn set_reward_fee(&mut self, reward_fee: u16) {
        self.assert_owner_calling();
        assert!(reward_fee < 10); // less than 10%
        self.rewards_fee_pct = reward_fee;
    }

    pub fn get_total_staked(&self) -> U128String {
        U128String::from(self.total_staked)
    }

    pub fn get_staking_key(&self) -> Base58PublicKey {
        panic!("No staking key for the staking pool");
    }

    pub fn is_staking_paused(&self) -> bool {
        self.staking_paused
    }

    pub fn get_account(&self, account_id: AccountId) -> HumanReadableAccount {
        let account = self.internal_get_account(&account_id);
        println!("account is {:?}", account);
        HumanReadableAccount {
            account_id,
            unstaked_balance: U128String::from(0), // TODO - implement unstake
            staked_balance: self.amount_from_stake_shares(account.stake_shares).into(),
            can_withdraw: false,
        }
    }

    pub fn get_number_of_accounts(&self) -> u64 {
        self.accounts.len()
    }

    pub fn get_accounts(&self, from_index: u64, limit: u64) -> Vec<HumanReadableAccount> {
        let keys = self.accounts.keys_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| self.get_account(keys.get(index).unwrap()))
            .collect()
    }

    // Contract state query
    pub fn get_nearx_pool_state(&self) -> NearxPoolStateResponse {
        NearxPoolStateResponse {
            owner_account_id: self.owner_account_id.clone(),
            contract_lock: self.contract_lock,
            staking_paused: self.staking_paused,
            contract_account_balance: U128String::from(self.contract_account_balance),
            total_staked: U128String::from(self.total_staked),
            total_stake_shares: U128String::from(self.total_stake_shares),
            accumulated_staked_rewards: U128String::from(self.accumulated_staked_rewards),
            min_deposit_amount: U128String::from(self.min_deposit_amount),
            operator_account_id: self.operator_account_id.clone(),
            rewards_fee_pct: U128String::from(self.rewards_fee_pct as u128),
        }
    }

    pub fn get_nearx_price(&self) -> U128String {
        self.amount_from_stake_shares(ONE_E24).into()
    }

    // Staking pool query
    pub fn get_validator_info(&self, inx: u16) -> ValidatorInfoResponse {
        assert!((inx as usize) < self.validators.len());
        let sp = &self.validators[inx as usize];

        ValidatorInfoResponse {
            inx,
            account_id: sp.account_id.clone(),
            staked: sp.staked.into(),
            last_asked_rewards_epoch_height: sp.last_redeemed_rewards_epoch.into(),
            lock: sp.lock,
        }
    }

    pub fn get_validators(&self) -> Vec<ValidatorInfoResponse> {
        let mut stake_pools_response = vec![];
        for i in 0..self.validators.len() {
            stake_pools_response.push(ValidatorInfoResponse {
                inx: i as u16,
                account_id: self.validators[i].account_id.clone(),
                staked: U128String::from(self.validators[i].staked),
                last_asked_rewards_epoch_height: U64String::from(
                    self.validators[i].last_redeemed_rewards_epoch,
                ),
                lock: self.validators[i].lock,
            });
        }

        stake_pools_response
    }
}
