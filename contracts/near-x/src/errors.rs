// TODO - refactor this to a module
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
pub const ERROR_VALIDATOR_IS_NOT_PRESENT: &str = "Validator is not present";
pub const ERROR_VALIDATOR_IS_ALREADY_PRESENT: &str = "Validator is already present in list";
pub const ERROR_NO_STAKING_KEY: &str = "Staking key not present";
pub const ERROR_CONTRACT_ALREADY_INITIALIZED: &str = "Contract has already been initialized";
pub const ERROR_NON_POSITIVE_UNSTAKE_AMOUNT: &str = "Unstake amount needs to be greater than 0";
pub const ERROR_NO_STAKED_BALANCE: &str = "Staked balance is 0";
pub const ERROR_NON_POSITIVE_UNSTAKING_SHARES: &str = "Unstaked shares should be greater than 0";
pub const ERROR_NON_POSITIVE_UNSTAKE_RECEVIE_AMOUNT: &str =
    "Received unstake amount needs to be greater than 0";
pub const ERROR_NON_POSITIVE_WITHDRAWAL: &str = "Withdrawal amount needs to be greater than 0";
pub const ERROR_NOT_ENOUGH_UNSTAKED_AMOUNT_TO_WITHDRAW: &str =
    "Not enough unstaked amount to withdraw";
pub const ERROR_UNSTAKED_AMOUNT_IN_UNBONDING_PERIOD: &str =
    "Unstaking amount still in unbonding period";
pub const ERROR_NOT_ENOUGH_BALANCE_FOR_STORAGE: &str = "Not enough balance for storage staking";
pub const ERROR_CANNOT_UNSTAKED_MORE_THAN_STAKED_AMOUNT: &str =
    "Cannot unstake more than staked amount";
pub const ERROR_NOT_ENOUGH_STAKED_AMOUNT_TO_UNSTAKE: &str = "Not enough staked amount to unstake";
pub const ERROR_NOT_ENOUGH_CONTRACT_STAKED_AMOUNT: &str = "Not enough staked amount in contract";
pub const ERROR_VALIDATOR_DOES_NOT_EXIST: &str = "Validator not exist in pool";
pub const ERROR_NOT_ENOUGH_GAS: &str = "Not enough pre-paid gas";
pub const ERROR_VALIDATOR_UNSTAKE_STILL_UNBONDING: &str =
    "Unstaked amount is still in unbonding period";
pub const ERROR_NO_VALIDATOR_AVAILABLE_FOR_UNSTAKE: &str =
    "No validator is available to unstake from";
pub const ERROR_NON_POSITIVE_STAKE_AMOUNT: &str = "Amount to stake must be greater than 0";
pub const ERROR_NON_POSITIVE_STAKE_SHARES: &str = "nearx to be minted must be greater than 0";
pub const ERROR_NO_VALIDATOR_AVAILABLE_TO_STAKE: &str = "No validator available to stake";