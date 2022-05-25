use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::AccountId;

pub type U128String = U128;
pub type U64String = U64;

/// Rewards fee fraction structure for the staking pool contract.
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct RewardFeeFraction {
    pub numerator: u32,
    pub denominator: u32,
}

#[allow(clippy::all)]
mod uint_impl {
    uint::construct_uint! {
        /// 256-bit unsigned integer.
        pub struct U256(4);
    }
}
pub use uint_impl::U256;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct HumanReadableAccount {
    pub account_id: AccountId,
    /// The unstaked balance that can be withdrawn or staked.
    pub unstaked_balance: U128,
    /// The amount balance staked at the current "stake" share price.
    pub staked_balance: U128,
    /// Whether the unstaked balance is available for withdrawal now.
    pub can_withdraw: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct NearxPoolStateResponse {
    pub owner_account_id: AccountId,

    /// Avoid re-entry when async-calls are in-flight
    pub contract_lock: bool,

    /// no auto-staking. true while changing staking pools
    pub staking_paused: bool,

    // The amount of NEAR in the contract
    pub contract_account_balance: U128,

    /// The total amount of tokens actually staked (the tokens are in the staking pools)
    pub total_staked: U128,

    /// how many "shares" were minted. Every time someone "stakes" he "buys pool shares" with the staked amount
    // the buy share price is computed so if she "sells" the shares on that moment she recovers the same near amount
    // staking produces rewards, rewards are added to total_for_staking so share_price will increase with rewards
    // share_price = total_for_staking/total_shares
    // when someone "unstakes" they "burns" X shares at current price to recoup Y near
    pub total_stake_shares: U128, //total NearX minted

    /// the staking pools will add rewards to the staked amount on each epoch
    /// here we store the accumulated amount only for stats purposes. This amount can only grow
    pub accumulated_staked_rewards: U128,

    /// min amount accepted as deposit or stake
    pub min_deposit_amount: U128,

    pub operator_account_id: AccountId,
    /// pct of rewards which will go to the operator
    pub rewards_fee_pct: U128,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct StakePoolInfoResponse {
    pub inx: u16,
    pub account_id: String,
    pub staked: U128String,
    pub last_asked_rewards_epoch_height: U64String,
    pub lock: bool,
}
