use crate::*;
use near_sdk::EpochHeight;

use crate::types::*;
use crate::utils::*;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::ext_contract;

#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct StakingPoolInfo {
    pub account_id: AccountId,

    //if we've made an async call to this pool
    pub lock: bool,

    //total staked here
    pub staked: u128,

    //EpochHeight where we asked the sp what were our staking rewards
    pub last_asked_rewards_epoch_height: EpochHeight,
}

impl StakingPoolInfo {
    pub fn is_empty(&self) -> bool {
        return self.lock == false && self.staked == 0;
    }

    pub fn new(account_id: AccountId) -> Self {
        return Self {
            account_id,
            lock: false,
            staked: 0,
            last_asked_rewards_epoch_height: 0,
        };
    }
    pub fn total_balance(&self) -> u128 {
        self.staked
    }
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
