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
pub const ERROR_VALIDATOR_IS_ALREADY_PRESENT: &str = "Validator is already present in list";
pub const ERROR_NO_STAKING_KEY: &str = "Staking key not present";
pub const ERROR_CONTRACT_ALREADY_INITIALIZED: &str = "Contract has already been initialized";

pub const VALIDATOR_IS_NOT_PRESENT: &str = "Validator is not present";
pub const NOT_ENOUGH_SHARES: &str = "User has not enough shares to unstake";
pub const UNSTAKE_AMOUNT_ZERO: &str = "Unstake amount should not be 0";
pub const VALIDATORS_ARE_BUSY: &str = "Not a single validator is available";
pub const NOT_ENOUGH_TOKEN_TO_WITHDRAW: &str = "User has not enough tokens to withdraw";
pub const TOKENS_ARE_NOT_READY_FOR_WITHDRAWAL: &str = "The cooldown has not be elapsed yet";
