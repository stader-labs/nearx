use near_sdk::EpochHeight;

/// The contract keeps at least 40 NEAR in the account to avoid being transferred out to cover
/// contract code storage and some internal state.
pub const MIN_BALANCE_FOR_STORAGE: u128 = 40_000_000_000_000_000_000_000_000;

pub const NO_DEPOSIT: u128 = 0;
pub const ONE_E24: u128 = 1_000_000_000_000_000_000_000_000;
pub const NEAR: u128 = ONE_E24;
pub const ONE_NEAR: u128 = NEAR;
pub const NEAR_CENT: u128 = NEAR / 100;
pub const ONE_MILLI_NEAR: u128 = NEAR / 1_000;
pub const ONE_MICRO_NEAR: u128 = NEAR / 1_000_000;
pub const TWO_NEAR: u128 = 2 * NEAR;
pub const FIVE_NEAR: u128 = 5 * NEAR;
pub const TEN_NEAR: u128 = 10 * NEAR;
pub const K_NEAR: u128 = 1_000 * NEAR;

pub const NUM_EPOCHS_TO_UNLOCK: EpochHeight = 4;

/// Storage keys
pub const ACCOUNTS_MAP: &str = "A";
pub const VALIDATOR_MAP: &str = "B";

pub mod gas {
    use near_sdk::Gas;

    /// Gas attached to deposit call on the staking pool contract.
    pub const DEPOSIT_AND_STAKE: Gas = base_gas(3);

    /// Gas attached to stake call on the staking pool contract.
    pub const STAKE: Gas = base_gas(3);

    /// The amount of gas required to get the current staked balance of this account from the
    /// staking pool.
    pub const GET_ACCOUNT_STAKED_BALANCE: Gas = base_gas(1);

    /// The amount of gas required to get current unstaked balance of this account from the
    /// staking pool.
    pub const GET_ACCOUNT_UNSTAKED_BALANCE: Gas = base_gas(1);

    /// The amount of gas required to get the current total balance of this account from the
    /// staking pool.
    pub const GET_ACCOUNT_TOTAL_BALANCE: Gas = base_gas(1);

    /// Gas attached to the inner callback for processing result of the deposit and stake call to
    /// the staking pool.
    pub const ON_STAKE_POOL_DEPOSIT_AND_STAKE: Gas = base_gas(1);

    pub const ON_STAKE_POOL_DEPOSIT_AND_STAKE_CB: Gas = base_gas(1);

    /// Gas attached to the inner callback for processing result of the call to get the current total balance from the staking pool.
    /// TODO - bchain - see if we can refactor this
    pub const ON_GET_SP_STAKED_BALANCE_TO_RECONCILE: Gas = tera(5);

    pub const ON_STAKE_POOL_WITHDRAW_ALL: Gas = base_gas(3);

    pub const ON_STAKE_POOL_WITHDRAW_ALL_CB: Gas = base_gas(3);

    pub const ON_STAKE_POOL_UNSTAKE: Gas = base_gas(3);

    pub const ON_STAKE_POOL_UNSTAKE_CB: Gas = base_gas(3);

    pub const WITHDRAW_EPOCH: Gas = base_gas(3);

    pub const UNSTAKE_EPOCH: Gas = base_gas(3);

    pub const STAKE_EPOCH: Gas = base_gas(3);

    const fn base_gas(n: u64) -> Gas {
        Gas(1_000_000_000 * 25 * n)
    }

    const fn tera(n: u64) -> Gas {
        Gas(1_000_000_000 * n)
    }
}
