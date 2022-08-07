use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::AccountId;
use near_sdk::EpochHeight;
use near_x::state::Fraction;
use workspaces::types::Balance;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LegacyNearxPoolStateResponse {
    pub owner_account_id: AccountId,

    /// The total amount of tokens actually staked (the tokens are in the staking pools)
    pub total_staked: U128,

    /// how many "shares" were minted. Every time someone "stakes" he "buys pool shares" with the staked amount
    // the buy share price is computed so if she "sells" the shares on that moment she recovers the same near amount
    // staking produces rewards, rewards are added to total_for_staking so share_price will increase with rewards
    // share_price = total_staked/total_shares
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

#[derive(Default, BorshDeserialize, BorshSerialize, Debug, PartialEq, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LegacyAccount {
    pub stake_shares: u128, //nearx this account owns

    pub unstaked_amount: Balance,

    pub withdrawable_epoch_height: EpochHeight,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LegacyRolesResponse {
    pub owner_account: AccountId,
    pub operator_account: AccountId,
    pub treasury_account: AccountId,
    pub temp_owner: Option<AccountId>,
}
