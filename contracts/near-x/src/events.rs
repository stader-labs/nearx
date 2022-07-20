use crate::contract::OperationControls;
use crate::state::Fraction;
use near_sdk::{json_types::U128, log, serde::Serialize, serde_json::json, AccountId};

const EVENT_STANDARD: &str = "nearx";
const EVENT_STANDARD_VERSION: &str = "1.0.0";

#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum Event {
    // Epoch Actions
    StakingEpochAttempt {
        validator_id: AccountId,
        amount: U128,
    },
    StakingEpochCallbackSuccess {
        validator_id: AccountId,
        amount: U128,
    },
    StakingEpochCallbackFailed {
        validator_id: AccountId,
        amount: U128,
    },
    UnstakingEpochAttempt {
        validator_id: AccountId,
        amount: U128,
    },
    UnstakingEpochCallbackSuccess {
        validator_id: AccountId,
        amount: U128,
    },
    UnstakingEpochCallbackFailed {
        validator_id: AccountId,
        amount: U128,
    },
    WithdrawEpochAttempt {
        validator_id: AccountId,
        amount: U128,
    },
    WithdrawEpochCallbackSuccess {
        validator_id: AccountId,
        amount: U128,
    },
    WithdrawEpochCallbackFailed {
        validator_id: AccountId,
        amount: U128,
    },
    AutocompoundingEpochRewardsAttempt {
        validator_id: AccountId,
    },
    AutocompoundingEpochRewards {
        validator_id: AccountId,
        old_balance: U128,
        new_balance: U128,
        rewards: U128,
    },
    EpochReconcile {
        actual_epoch_stake_amount: U128,
        actual_epoch_unstake_amount: U128,
        reconciled_stake_amount: U128,
        reconciled_unstake_amount: U128,
    },
    // Sync validator balance
    BalanceSyncedFromValidatorAttempt {
        validator_id: AccountId,
    },
    BalanceSyncedFromValidator {
        validator_id: AccountId,
        old_staked_balance: U128,
        old_unstaked_balance: U128,
        staked_balance: U128,
        unstaked_balance: U128,
    },
    // Staking Pool Interface
    DepositAndStake {
        account_id: AccountId,
        amount: U128,
        minted_stake_shares: U128,
        new_stake_shares: U128,
    },
    Withdraw {
        account_id: AccountId,
        amount: U128,
        new_unstaked_balance: U128,
    },
    Unstake {
        account_id: AccountId,
        unstaked_amount: U128,
        burnt_stake_shares: U128,
        new_unstaked_balance: U128,
        new_stake_shares: U128,
        unstaked_available_epoch_height: u64,
    },
    // Validators
    ValidatorAdded {
        account_id: AccountId,
        weight: u16,
    },
    ValidatorRemoved {
        account_id: AccountId,
    },
    ValidatorUpdated {
        account_id: AccountId,
        weight: u16,
    },
    ValidatorPaused {
        account_id: AccountId,
        old_weight: u16,
    },
    // Validator draining
    DrainUnstake {
        account_id: AccountId,
        amount: U128,
    },
    DrainUnstakeCallbackFail {
        validator_id: AccountId,
        amount: U128,
    },
    DrainUnstakeCallbackSuccess {
        validator_id: AccountId,
        amount: U128,
    },
    DrainWithdraw {
        validator_id: AccountId,
        amount: U128,
    },
    DrainWithdrawCallbackFail {
        validator_id: AccountId,
        amount: U128,
    },
    DrainWithdrawCallbackSuccess {
        validator_id: AccountId,
        amount: U128,
    },
    // Ft related events
    FtTransfer {
        receiver_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    },
    FtTransferCall {
        receiver_id: AccountId,
        sender_id: AccountId,
        msg: String,
        amount: U128,
    },
    FtBurn {
        account_id: AccountId,
        amount: U128,
    },
    // Owner events
    SetOwner {
        old_owner: AccountId,
        new_owner: AccountId,
    },
    CommitOwner {
        new_owner: AccountId,
        caller: AccountId,
    },
    UpdateOperator {
        old_operator: AccountId,
        new_operator: AccountId,
    },
    UpdateTreasury {
        old_treasury_account: AccountId,
        new_treasury_account: AccountId,
    },
    UpdateOperationsControl {
        operations_control: OperationControls,
    },
    SetRewardFee {
        old_reward_fee: Fraction,
        new_reward_fee: Fraction,
    },
    SetMinDeposit {
        old_min_deposit: U128,
        new_min_deposit: U128,
    },
}

impl Event {
    pub fn emit(&self) {
        emit_event(&self);
    }
}

// Emit event that follows NEP-297 standard: https://nomicon.io/Standards/EventsFormat
// Arguments
// * `standard`: name of standard, e.g. nep171
// * `version`: e.g. 1.0.0
// * `event`: type of the event, e.g. nft_mint
// * `data`: associate event data. Strictly typed for each set {standard, version, event} inside corresponding NEP
pub(crate) fn emit_event<T: ?Sized + Serialize>(data: &T) {
    let result = json!(data);
    let event_json = json!({
        "standard": EVENT_STANDARD,
        "version": EVENT_STANDARD_VERSION,
        "event": result["event"],
        "data": [result["data"]]
    })
    .to_string();
    log!(format!("EVENT_JSON:{}", event_json));
}
