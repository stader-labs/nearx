// TODO - bchain - refactor the codebase to consolidate errors in one place
pub const ERROR_VALIDATOR_IS_BUSY: &str = "Validator is busy";
pub const ERROR_DEPOSIT_SHOULD_BE_GREATER_THAN_ZERO: &str = "Deposit should be greater than 0";
pub const ERROR_UNAUTHORIZED: &str = "Unauthorized";
pub const ERROR_CONTRACT_BUSY: &str = "Contract is busy";
pub const ERROR_STAKING_PAUSED: &str = "Staking paused";
pub const ERROR_MIN_DEPOSIT: &str = "Deposit should be greater than min deposit";
pub const ERROR_MIN_BALANCE_FOR_CONTRACT_STORAGE: &str =
    "Contract balance should not go below the required minimum storage balance";
pub const ERROR_REQUIRE_ONE_YOCTO_NEAR: &str = "Function requires at least one yocto near";
pub const ERROR_EXPECT_RESULT_ON_CALLBACK: &str = "Callback expected result on callback";
