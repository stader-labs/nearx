// TODO - bchain - There are some issues when i try to add the near-liquid-token repo to the Cargo.toml. let's fix it later
// For now we can copy the types over here

// pub type U128String = U128;
// pub type U64String = U64;
use serde::{Deserialize, Serialize};
use workspaces::AccountId;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
// #[serde(crate = "near_sdk::serde")]
pub struct ValidatorInfoResponse {
    pub inx: u16,
    pub account_id: String,
    pub staked: String,
    pub last_asked_rewards_epoch_height: String,
    pub lock: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct AccountResponse {
    pub account_id: AccountId,
    /// The unstaked balance that can be withdrawn or staked.
    pub unstaked_balance: String,
    /// The amount balance staked at the current "stake" share price.
    pub staked_balance: String,
    /// Whether the unstaked balance is available for withdrawal now.
    pub can_withdraw: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NearxPoolStateResponse {
    pub owner_account_id: AccountId,

    /// Avoid re-entry when async-calls are in-flight
    pub contract_lock: bool,

    /// no auto-staking. true while changing staking pools
    pub staking_paused: bool,

    // The amount of NEAR in the contract
    pub contract_account_balance: String,

    /// The total amount of tokens actually staked (the tokens are in the staking pools)
    pub total_staked: String,

    /// how many "shares" were minted. Every time someone "stakes" he "buys pool shares" with the staked amount
    // the buy share price is computed so if she "sells" the shares on that moment she recovers the same near amount
    // staking produces rewards, rewards are added to total_for_staking so share_price will increase with rewards
    // share_price = total_for_staking/total_shares
    pub total_stake_shares: String, //total NearX minted

    /// the staking pools will add rewards to the staked amount on each epoch
    /// here we store the accumulated amount only for stats purposes. This amount can only grow
    pub accumulated_staked_rewards: String,

    /// min amount accepted as deposit or stake
    pub min_deposit_amount: String,

    pub operator_account_id: AccountId,
    /// pct of rewards which will go to the operator
    pub rewards_fee_pct: String,
}
