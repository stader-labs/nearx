use near_sdk::{json_types::U128, log, serde::Serialize, serde_json::json, AccountId};

const EVENT_STANDARD: &str = "linear";
const EVENT_STANDARD_VERSION: &str = "1.0.0";

#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum Event {
    // Epoch Actions
    EpochStakeAttempt {
        validator_id: AccountId,
        amount: U128,
    },
    EpochStakeCallbackSuccess {
        validator_id: AccountId,
        amount: U128,
    },
    EpochStakeCallbackFailed {
        validator_id: AccountId,
        amount: U128,
    },
    EpochUnstakeAttempt {
        validator_id: AccountId,
        amount: U128,
    },
    EpochUnstakeCallbackSuccess {
        validator_id: AccountId,
        amount: U128,
    },
    EpochUnstakeCallbackFailed {
        validator_id: AccountId,
        amount: U128,
    },
    EpochWithdrawAttempt {
        validator_id: AccountId,
        amount: U128,
    },
    EpochWithdrawCallbackSuccess {
        validator_id: AccountId,
        amount: U128,
    },
    EpochWithdrawCallbackFailed {
        validator_id: AccountId,
        amount: U128,
    },
    EpochAutocompoundRewardsAttempt {
        validator_id: AccountId,
    },
    EpochAutocompoundRewards {
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
    },
    ValidatorRemoved {
        account_id: AccountId,
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
        amount: U128
    },
    FtTransferCall {
        receiver_id: AccountId,
        sender_id: AccountId,
        msg: String,
        amount: U128,
    },
    FtBurn {
        account_id: AccountId,
        amount: U128
    }
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
