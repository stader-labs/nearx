use near_sdk::{json_types::U128, log, serde::Serialize, serde_json::json, AccountId};

const EVENT_STANDARD: &str = "nearx";
const EVENT_STANDARD_VERSION: &str = "1.0.0";

#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum Event {
    // Epoch Actions
    EpochStake {
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
    EpochUnstake {
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
    EpochWithdraw {
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
    EpochAutocompoundRewards {
        validator_id: AccountId,
        old_balance: U128,
        new_balance: U128,
        rewards: U128,
    },
    EpochReconcilation {
        stake_amount_to_settle: U128,
        unstake_amount_to_settle: U128,
    },
    // Staking Pool Interface
    Withdraw {
        account_id: AccountId,
        amount: U128,
        new_unstaked_balance: U128,
    },
    DepositAndStake {
        account_id: AccountId,
        staked_amount: U128,
        minted_stake_shares: U128,
        new_unstaked_balance: U128,
        new_stake_shares: U128,
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
}

impl Event {
    pub fn emit(&self) {
        emit_event(&self);
    }
}

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
