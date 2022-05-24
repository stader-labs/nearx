use crate::constants::MIN_BALANCE_FOR_STORAGE;
use crate::errors::*;
use crate::types::*;
use near_sdk::json_types::U128;
use near_sdk::{env, PromiseResult};

pub fn assert_min_balance(amount: u128) {
    assert!(amount > 0, ERROR_DEPOSIT_SHOULD_BE_GREATER_THAN_ZERO);
    assert!(
        env::account_balance() >= MIN_BALANCE_FOR_STORAGE
            && env::account_balance() - MIN_BALANCE_FOR_STORAGE > amount,
        ERROR_MIN_BALANCE_FOR_CONTRACT_STORAGE
    );
}

pub fn assert_callback_calling() {
    assert_eq!(env::predecessor_account_id(), env::current_account_id());
}

pub fn assert_one_yocto() {
    assert_eq!(
        env::attached_deposit(),
        1,
        "the function requires 1 yocto attachment"
    );
}

pub fn is_promise_success() -> bool {
    assert_eq!(
        env::promise_results_count(),
        1,
        "Contract expected a result on the callback"
    );
    match env::promise_result(0) {
        PromiseResult::Successful(_) => true,
        _ => false,
    }
}

pub fn apply_multiplier(amount: u128, percentage: u16) -> u128 {
    return (amount.checked_mul(percentage as u128))
        .unwrap()
        .checked_div(100)
        .unwrap();
}
/// returns amount * numerator/denominator
pub fn proportional(amount: u128, numerator: u128, denominator: u128) -> u128 {
    return (U256::from(amount) * U256::from(numerator) / U256::from(denominator)).as_u128();
}

/// Returns the number of shares corresponding to the given near amount at current share_price
/// if the amount & the shares are incorporated, price remains the same
pub fn shares_from_amount(amount: u128, total_amount: u128, total_shares: u128) -> u128 {
    if total_shares == 0 {
        return amount;
    }
    if amount == 0 || total_amount == 0 {
        return 0;
    }
    return proportional(total_shares, amount, total_amount);
}

pub fn amount_from_shares(num_shares: u128, total_amount: u128, total_shares: u128) -> u128 {
    if total_shares == 0 || num_shares == 0 {
        return 0;
    };
    return proportional(num_shares, total_amount, total_shares);
}
