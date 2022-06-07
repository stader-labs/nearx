use crate::{
    constants::MIN_BALANCE_FOR_STORAGE,
    errors::{self, *},
    state::ValidatorInfo,
};
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
        "{}",
        ERROR_REQUIRE_ONE_YOCTO_NEAR
    );
}

pub fn is_promise_success() -> bool {
    assert_eq!(
        env::promise_results_count(),
        1,
        "{}",
        ERROR_EXPECT_RESULT_ON_CALLBACK
    );

    matches!(env::promise_result(0), PromiseResult::Successful(_))
}

/// Returns amount * numerator/denominator
#[allow(clippy::all)]
pub fn proportional(amount: u128, numerator: u128, denominator: u128) -> u128 {
    uint::construct_uint! {
        /// 256-bit unsigned integer.
        pub struct U256(4);
    }

    (U256::from(amount) * U256::from(numerator) / U256::from(denominator)).as_u128()
}
