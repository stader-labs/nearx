use crate::constants::NUM_EPOCHS_TO_UNLOCK;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::{U128, U64},
    serde::{Deserialize, Serialize},
    AccountId, Balance, EpochHeight,
};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountResponse {
    pub account_id: AccountId,
    pub unstaked_balance: U128,
    pub staked_balance: U128,
    pub withdrawable_epoch: U64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct NearxPoolStateResponse {
    pub owner_account_id: AccountId,

    pub contract_lock: bool,

    pub staking_paused: bool,

    /// The total amount of tokens actually staked (the tokens are in the staking pools)
    pub total_staked: U128,

    /// how many "shares" were minted. Every time someone "stakes" he "buys pool shares" with the staked amount
    // the buy share price is computed so if she "sells" the shares on that moment she recovers the same near amount
    // staking produces rewards, rewards are added to total_for_staking so share_price will increase with rewards
    // share_price = total_for_staking/total_shares
    pub total_stake_shares: U128, //total NearX minted

    pub accumulated_staked_rewards: U128,

    /// min amount accepted as deposit or stake
    pub min_deposit_amount: U128,

    pub operator_account_id: AccountId,

    /// pct of rewards which will go to the operator
    pub rewards_fee_pct: Fraction,

    /// Amount of NEAR that is users requested to stake
    pub user_amount_to_stake_in_epoch: U128,
    /// Amount of NEAR that is users requested to unstake
    pub user_amount_to_unstake_in_epoch: U128,

    /// Amount of NEAR that actually needs to be staked in the epoch
    pub reconciled_epoch_stake_amount: U128,
    /// Amount of NEAR that actually needs to be unstaked in the epoch
    pub reconciled_epoch_unstake_amount: U128,
    /// Last epoch height stake/unstake amount were reconciled
    pub last_reconcilation_epoch: U64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct ValidatorInfoResponse {
    pub account_id: AccountId,
    pub staked: U128,
    pub unstaked: U128,
    pub last_asked_rewards_epoch_height: U64,
    pub last_unstake_start_epoch: U64,
    pub lock: bool,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ValidatorInfo {
    pub account_id: AccountId,

    // TODO - bchain99 - we might not need this
    pub lock: bool,

    pub staked: u128,

    pub last_redeemed_rewards_epoch: EpochHeight,

    pub unstaked_amount: Balance,

    pub unstake_start_epoch: EpochHeight,

    pub last_unstake_start_epoch: EpochHeight,
}

impl ValidatorInfo {
    pub fn is_empty(&self) -> bool {
        self.unlocked() && self.staked == 0
    }

    pub fn new(account_id: AccountId) -> Self {
        Self {
            account_id,
            lock: false,
            staked: 0,
            last_redeemed_rewards_epoch: 0,
            unstaked_amount: 0,
            unstake_start_epoch: 0,
            last_unstake_start_epoch: 0,
        }
    }

    pub fn total_balance(&self) -> u128 {
        self.staked
    }

    pub fn unlocked(&self) -> bool {
        self.lock == false
    }

    /// whether the validator is in unstake releasing period.
    pub fn pending_unstake_release(&self) -> bool {
        let current_epoch = env::epoch_height();
        current_epoch >= self.unstake_start_epoch
            && current_epoch < self.unstake_start_epoch + NUM_EPOCHS_TO_UNLOCK
    }
}

#[derive(Default, BorshDeserialize, BorshSerialize, Debug, PartialEq, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Account {
    pub stake_shares: u128, //nearx this account owns

    pub unstaked_amount: Balance,

    pub withdrawable_epoch_height: EpochHeight,
}

impl Account {
    pub fn is_empty(&self) -> bool {
        self.stake_shares == 0 && self.unstaked_amount == 0
    }

    pub fn add_stake_shares(&mut self, num_shares: u128) {
        self.stake_shares += num_shares;
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

/// Rewards fee fraction structure for the staking pool contract.
#[derive(Debug, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Copy)]
#[serde(crate = "near_sdk::serde")]
pub struct Fraction {
    pub numerator: u32,
    pub denominator: u32,
}

impl Fraction {
    pub fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

impl std::ops::Mul<Fraction> for u128 {
    type Output = u128;

    fn mul(self, rhs: Fraction) -> Self::Output {
        crate::utils::proportional(self, rhs.numerator.into(), rhs.denominator.into())
    }
}
