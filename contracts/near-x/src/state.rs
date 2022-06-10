use crate::constants::UNSTAKE_COOLDOWN_EPOCH;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env,
    json_types::{U128, U64},
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, Balance, EpochHeight, PanicOnDefault,
};

pub use response::*;

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

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ValidatorInfo {
    pub account_id: AccountId,

    // TODO - bchain99 - we might not need this
    pub lock: bool,

    pub staked: u128,

    /// Amount of unstaked tokens that are ready for withdrawal
    /// when the current epoch is at `available_for_unstake`.
    pub to_withdraw: u128,

    pub last_redeemed_rewards_epoch: EpochHeight,

    /// The epoch when we can run the unstake instruction again.
    available_for_unstake: EpochHeight,
}

#[derive(Default, BorshDeserialize, BorshSerialize, Debug, PartialEq, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Account {
    /// NearX this account owns.
    pub stake_shares: u128,
    /// How many NEAR this account can withdraw.
    pub unstaked: u128,
    /// When the user is allowed to withdraw.
    pub withdrawable_epoch: EpochHeight,
}

/// Rewards fee fraction structure for the staking pool contract.
#[derive(Debug, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Copy)]
#[serde(crate = "near_sdk::serde")]
pub struct Fraction {
    pub numerator: u32,
    pub denominator: u32,
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
            to_withdraw: 0,
            last_redeemed_rewards_epoch: 0,
            available_for_unstake: 0,
        }
    }

    pub fn total_balance(&self) -> u128 {
        self.staked
    }

    pub fn unlocked(&self) -> bool {
        self.lock == false
    }

    pub fn make_unavailable(&mut self) {
        self.available_for_unstake = env::epoch_height() + UNSTAKE_COOLDOWN_EPOCH;
    }

    /// Returns whether the validator is available for unstaking at that epoch, or not.
    pub fn available(&self) -> bool {
        env::epoch_height() > self.available_for_unstake
    }
}

impl Account {
    pub fn is_empty(&self) -> bool {
        self.stake_shares == 0 && self.unstaked == 0
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

    pub fn cooldown_finished(&self) -> bool {
        env::epoch_height() > self.withdrawable_epoch
    }
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

mod response {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(crate = "near_sdk::serde")]
    pub struct AccountResponse {
        pub account_id: AccountId,
        pub unstaked_balance: U128,
        pub staked_balance: U128,
        pub stake_shares: U128,
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

        pub user_amount_to_stake_in_epoch: U128,

        /// min amount accepted as deposit or stake
        pub min_deposit_amount: U128,

        pub operator_account_id: AccountId,

        /// pct of rewards which will go to the operator
        pub rewards_fee_pct: Fraction,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(crate = "near_sdk::serde")]
    pub struct ValidatorInfoResponse {
        pub account_id: AccountId,
        pub staked: U128,
        pub last_asked_rewards_epoch_height: U64,
        pub lock: bool,
    }
}
