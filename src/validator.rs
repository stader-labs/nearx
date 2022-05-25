use crate::*;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    ext_contract, EpochHeight,
};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ValidatorInfo {
    pub account_id: AccountId,

    // TODO - bchain99 - we might not need this
    pub lock: bool,

    pub staked: u128,

    pub last_redeemed_rewards_epoch: EpochHeight,
}

impl ValidatorInfo {
    pub fn is_empty(&self) -> bool {
        self.lock == false && self.staked == 0
    }

    pub fn new(account_id: AccountId) -> Self {
        Self {
            account_id,
            lock: false,
            staked: 0,
            last_redeemed_rewards_epoch: 0,
        }
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
