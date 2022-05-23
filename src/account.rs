use crate::*;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::log;
use near_sdk::serde::{Deserialize, Serialize};

use crate::types::*;
use crate::utils::*;

#[derive(Default, BorshDeserialize, BorshSerialize, Debug, PartialEq, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Account {
    pub stake_shares: u128, //nearx this account owns
}

impl Account {
    pub fn is_empty(&self) -> bool {
        return self.stake_shares == 0;
    }

    pub fn add_nearx(&mut self, nearx_amount: u128) {
        self.add_stake_shares(nearx_amount)
    }

    pub fn add_stake_shares(&mut self, num_shares: u128) {
        self.stake_shares += num_shares;
    }

    pub fn sub_nearx(&mut self, nearx_amount: u128) {
        self.sub_stake_shares(nearx_amount)
    }

    pub fn sub_stake_shares(&mut self, num_shares: u128) {
        assert!(
            self.stake_shares >= num_shares,
            "sub_stake_shares self.stake_shares {} < num_shares {}",
            self.stake_shares,
            num_shares
        );
        self.stake_shares -= num_shares;
    }
}
