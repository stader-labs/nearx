// TODO - bchain - There are some issues when i try to add the near-liquid-token repo to the Cargo.toml. let's fix it later
// For now we can copy the types over here

// pub type U128String = U128;
// pub type U64String = U64;
use serde::{Serialize, Deserialize};
use workspaces::AccountId;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
// #[serde(crate = "near_sdk::serde")]
pub struct StakePoolInfoResponse {
    pub inx: u16,
    pub account_id: String,
    pub staked: String,
    pub last_asked_rewards_epoch_height: String,
    pub lock: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct HumanReadableAccount {
    pub account_id: AccountId,
    /// The unstaked balance that can be withdrawn or staked.
    pub unstaked_balance: String,
    /// The amount balance staked at the current "stake" share price.
    pub staked_balance: String,
    /// Whether the unstaked balance is available for withdrawal now.
    pub can_withdraw: bool,
}