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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
#[serde(rename_all = "camelCase")]
pub struct SnapshotUser {
    pub account_id: AccountId,
    pub nearx_balance: U128,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(rename_all = "camelCase")]
pub enum ValidatorType {
    PUBLIC,
    PRIVATE,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct NearxPoolStateResponse {
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

    pub temp_reward_fee: Option<Fraction>,

    pub rewards_buffer: U128,

    pub accumulated_rewards_buffer: U128,

    pub last_reward_fee_set_epoch: EpochHeight,

    pub min_storage_reserve: U128,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(rename_all = "camelCase")]
pub enum ValidatorInfoWrapper {
    LegacyValidatorInfo(LegacyValidatorInfo),
    ValidatorInfo(ValidatorInfo),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct ValidatorInfoResponse {
    pub account_id: AccountId,
    pub staked: U128,
    pub unstaked: U128,
    pub weight: u16,
    pub last_asked_rewards_epoch_height: U64,
    pub last_unstake_start_epoch: U64,
    pub max_unstakable_limit: Option<u128>,
    pub validator_type: ValidatorType,
    pub redelegate_to: Option<AccountId>,
    pub amount_to_redelegate: U128,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct LegacyValidatorInfo {
    pub account_id: AccountId,

    pub staked: u128,

    pub weight: u16,

    pub last_redeemed_rewards_epoch: EpochHeight,

    pub unstaked_amount: Balance,

    pub unstake_start_epoch: EpochHeight,

    pub last_unstake_start_epoch: EpochHeight,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ValidatorInfo {
    pub account_id: AccountId,

    pub staked: u128,

    pub weight: u16,

    pub last_redeemed_rewards_epoch: EpochHeight,

    pub unstaked_amount: Balance,

    pub unstake_start_epoch: EpochHeight,

    pub last_unstake_start_epoch: EpochHeight,

    // the max amount that can be unstaked from a validator at a given time
    // if None, then it means that we can unstake the entire amount from the validator
    pub max_unstakable_limit: Option<u128>,

    pub validator_type: ValidatorType,

    pub redelegate_to: Option<AccountId>,

    pub amount_to_redelegate: u128,
}

impl ValidatorInfo {
    pub fn is_empty(&self) -> bool {
        self.paused()
            && !self.pending_unstake_release()
            && self.staked == 0
            && self.unstaked_amount == 0
    }

    pub fn new(account_id: AccountId, weight: u16) -> Self {
        Self {
            account_id,
            staked: 0,
            weight,
            last_redeemed_rewards_epoch: 0,
            unstaked_amount: 0,
            unstake_start_epoch: 0,
            last_unstake_start_epoch: 0,
            max_unstakable_limit: None,
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: 0,
        }
    }

    pub fn total_balance(&self) -> u128 {
        self.staked + self.unstaked_amount
    }

    pub fn paused(&self) -> bool {
        self.weight == 0
    }

    /// whether the validator is in unstake releasing period.
    pub fn pending_unstake_release(&self) -> bool {
        env::epoch_height() >= self.unstake_start_epoch
            && env::epoch_height() < self.unstake_start_epoch + NUM_EPOCHS_TO_UNLOCK
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

#[derive(Serialize, Deserialize)]
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
pub struct RolesResponse {
    pub owner_account: AccountId,
    pub operator_account: AccountId,
    pub treasury_account: AccountId,
    pub temp_owner: Option<AccountId>,
    pub temp_operator: Option<AccountId>,
    pub temp_treasury: Option<AccountId>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct OperationsControlUpdateRequest {
    pub stake_paused: Option<bool>,
    pub direct_stake_paused: Option<bool>,
    pub unstake_paused: Option<bool>,
    pub withdraw_paused: Option<bool>,
    pub staking_epoch_paused: Option<bool>,
    pub unstaking_epoch_paused: Option<bool>,
    pub withdraw_epoch_paused: Option<bool>,
    pub autocompounding_epoch_paused: Option<bool>,
    pub sync_validator_balance_paused: Option<bool>,
    pub ft_transfer_paused: Option<bool>,
    pub ft_transfer_call_paused: Option<bool>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ContractSummary {
    pub total_staked: U128,
    pub total_shares: U128,
    pub total_validators: U128,
    pub treasury_staked_balance: U128,
    pub treasury_unstaked_balance: U128,
    pub nearx_price: U128,
}
