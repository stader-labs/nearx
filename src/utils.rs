use crate::constants::MIN_BALANCE_FOR_STORAGE;
use crate::errors::*;
use crate::types::*;
use near_sdk::{env, PromiseResult};

pub fn assert_min_balance(amount: u128) {
    assert!(amount > 0, "{}", ERROR_DEPOSIT_SHOULD_BE_GREATER_THAN_ZERO);
    assert!(
        env::account_balance() >= MIN_BALANCE_FOR_STORAGE
            && env::account_balance() - MIN_BALANCE_FOR_STORAGE > amount,
        "{}",
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
        "{}", ERROR_REQUIRE_ONE_YOCTO_NEAR
    );
}

pub fn is_promise_success() -> bool {
    assert_eq!(
        env::promise_results_count(),
        1,
        "{}", ERROR_EXPECT_RESULT_ON_CALLBACK
    );

    matches!(env::promise_result(0), PromiseResult::Successful(_))
}

pub fn apply_multiplier(amount: u128, percentage: u16) -> u128 {
    (amount.checked_mul(percentage as u128))
        .unwrap()
        .checked_div(100)
        .unwrap()
}
/// returns amount * numerator/denominator
pub fn proportional(amount: u128, numerator: u128, denominator: u128) -> u128 {
    (U256::from(amount) * U256::from(numerator) / U256::from(denominator)).as_u128()
}

pub fn shares_from_amount(amount: u128, total_amount: u128, total_shares: u128) -> u128 {
    if total_shares == 0 {
        return amount;
    }
    if amount == 0 || total_amount == 0 {
        return 0;
    }
    proportional(total_shares, amount, total_amount)
}

pub fn amount_from_shares(num_shares: u128, total_amount: u128, total_shares: u128) -> u128 {
    if total_shares == 0 || num_shares == 0 {
        0
    } else {
        proportional(num_shares, total_amount, total_shares)
    }
}
