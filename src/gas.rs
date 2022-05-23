use crate::constants::TGAS;

pub const BASE_GAS: u64 = 25 * TGAS;

pub mod staking_pool {
    /// Gas attached to deposit call on the staking pool contract.
    pub const DEPOSIT_AND_STAKE: u64 = super::BASE_GAS * 3;

    /// Gas attached to stake call on the staking pool contract.
    pub const STAKE: u64 = super::BASE_GAS * 3;

    /// The amount of gas required to get the current staked balance of this account from the
    /// staking pool.
    pub const GET_ACCOUNT_STAKED_BALANCE: u64 = super::BASE_GAS;

    /// The amount of gas required to get current unstaked balance of this account from the
    /// staking pool.
    pub const GET_ACCOUNT_UNSTAKED_BALANCE: u64 = super::BASE_GAS;

    /// The amount of gas required to get the current total balance of this account from the
    /// staking pool.
    pub const GET_ACCOUNT_TOTAL_BALANCE: u64 = super::BASE_GAS;
}

pub mod owner_callbacks {
    use crate::gas::TGAS;

    /// Gas attached to the inner callback for processing result of the deposit and stake call to
    /// the staking pool.
    pub const ON_STAKING_POOL_DEPOSIT_AND_STAKE: u64 = super::BASE_GAS;

    /// Gas attached to the inner callback for processing result of the call to get the current total balance from the staking pool.
    /// TODO - bchain - see if we can refactor this
    pub const ON_GET_SP_STAKED_BALANCE_TO_RECONCILE: u64 = 5 * TGAS;
}
