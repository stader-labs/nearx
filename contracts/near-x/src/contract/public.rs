use crate::errors::{
    self, ERROR_CONTRACT_ALREADY_INITIALIZED, ERROR_NO_STAKING_KEY,
    ERROR_VALIDATOR_IS_ALREADY_PRESENT,
};
use crate::{
    constants::{ACCOUNTS_MAP, MIN_STAKE_AMOUNT, ONE_E24, VALIDATOR_MAP},
    contract::*,
    state::*,
};
use near_sdk::{json_types::U64, log, near_bindgen, require, ONE_NEAR};

#[near_bindgen]
impl NearxPool {
    #[init]
    pub fn new(owner_account_id: AccountId, operator_account_id: AccountId) -> Self {
        require!(!env::state_exists(), ERROR_CONTRACT_ALREADY_INITIALIZED);

        Self {
            owner_account_id,
            contract_lock: false,
            operator_account_id,
            staking_paused: false,
            to_withdraw: 0,
            accumulated_staked_rewards: 0,
            total_stake_shares: 0,
            accounts: UnorderedMap::new(ACCOUNTS_MAP.as_bytes()),
            min_deposit_amount: ONE_NEAR,
            validator_info_map: UnorderedMap::new(VALIDATOR_MAP.as_bytes()),
            total_staked: 0,
            rewards_fee: Fraction::new(0, 1),
            last_reconcilation_epoch: 0,
            user_amount_to_stake_unstake: Direction::Stake(0),
            stake_unstake_locked_in_epoch: Direction::Stake(0),
        }
    }

    /// Rewards claiming
    pub fn ping(&mut self) {}
}

#[near_bindgen]
impl ExtStakingPool for NearxPool {
    fn get_account_staked_balance(&self, account_id: AccountId) -> U128 {
        self.get_account(account_id).staked_balance
    }

    fn get_account_unstaked_balance(&self, account_id: AccountId) -> U128 {
        self.internal_get_account(&account_id).unstaked.into()
    }

    fn get_account_total_balance(&self, account_id: AccountId) -> U128 {
        (self.get_account_staked_balance(account_id.clone()).0
            + self.get_account_unstaked_balance(account_id).0)
            .into()
    }

    #[payable]
    fn deposit(&mut self) {
        unimplemented!();
    }

    #[payable]
    fn deposit_and_stake_direct_stake(&mut self) {
        self.internal_deposit_and_stake_direct_stake(env::attached_deposit());
    }

    /// Deposits the attached amount into the inner account of the predecessor and stakes it.
    #[payable]
    fn deposit_and_stake(&mut self) {
        self.internal_deposit_and_stake(env::attached_deposit());
    }

    fn stake(&mut self, amount: U128) {
        let _ = amount;
        unimplemented!();
    }

    fn withdraw(&mut self, near_amount: U128) {
        self.internal_withdraw(Some(near_amount.0))
    }

    fn withdraw_all(&mut self) {
        self.internal_withdraw(None)
    }

    fn unstake(&mut self, near_amount: U128) {
        self.internal_unstake(Some(near_amount.0))
    }

    fn unstake_all(&mut self) {
        self.internal_unstake(None)
    }
}

/// Staking pool addition and deletion.
#[near_bindgen]
impl NearxPool {
    pub fn remove_validator(&mut self, validator: AccountId) {
        self.assert_operator_or_owner();
        log!(format!("Removing validator {}", validator));
        self.validator_info_map.remove(&validator);
    }

    pub fn add_validator(&mut self, validator: AccountId) {
        self.assert_operator_or_owner();
        if self.validator_info_map.get(&validator).is_some() {
            panic!("{}", ERROR_VALIDATOR_IS_ALREADY_PRESENT);
        }
        log!(format!("Adding validator {}", validator));
        self.validator_info_map
            .insert(&validator.clone(), &ValidatorInfo::new(validator));
    }

    pub fn toggle_staking_pause(&mut self) {
        self.assert_operator_or_owner();
        self.staking_paused = !self.staking_paused;
    }
}

/// View methods.
#[near_bindgen]
impl NearxPool {
    pub fn get_owner_id(&self) -> AccountId {
        self.owner_account_id.clone()
    }

    pub fn get_reward_fee_fraction(&self) -> Fraction {
        self.rewards_fee
    }

    pub fn set_reward_fee(&mut self, numerator: u32, denominator: u32) {
        self.assert_owner_calling();
        require!((numerator * 100 / denominator) < 20); // less than 20%
        self.rewards_fee = Fraction::new(numerator, denominator);
    }

    pub fn get_total_staked(&self) -> U128 {
        U128::from(self.total_staked)
    }

    pub fn get_staking_key(&self) -> PublicKey {
        panic!("{}", ERROR_NO_STAKING_KEY);
    }

    pub fn is_staking_paused(&self) -> bool {
        self.staking_paused
    }

    pub fn get_account(&self, account_id: AccountId) -> AccountResponse {
        let account = self.internal_get_account(&account_id);
        AccountResponse {
            account_id,
            unstaked_balance: U128::from(0), // TODO - implement unstake//
            staked_balance: self.amount_from_stake_shares(account.stake_shares).into(),
            stake_shares: account.stake_shares.into(),
            withdrawable_epoch: account.withdrawable_epoch.into(),
        }
    }

    pub fn get_number_of_accounts(&self) -> u64 {
        self.accounts.len()
    }

    pub fn get_accounts(&self, from_index: u64, limit: u64) -> Vec<AccountResponse> {
        let keys = self.accounts.keys_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| self.get_account(keys.get(index).unwrap()))
            .collect()
    }

    // Contract state query
    pub fn get_nearx_pool_state(&self) -> NearxPoolStateResponse {
        NearxPoolStateResponse {
            owner_account_id: self.owner_account_id.clone(),
            contract_lock: self.contract_lock,
            staking_paused: self.staking_paused,
            total_staked: U128(self.total_staked),
            total_stake_shares: U128(self.total_stake_shares),
            accumulated_staked_rewards: U128(self.accumulated_staked_rewards),
            min_deposit_amount: U128(self.min_deposit_amount),
            operator_account_id: self.operator_account_id.clone(),
            rewards_fee_pct: self.rewards_fee,
            last_reconcilation_epoch: U64(self.last_reconcilation_epoch),
        }
    }

    pub fn get_nearx_price(&self) -> U128 {
        let amount = self.amount_from_stake_shares(ONE_E24);

        if amount == 0 {
            U128(ONE_E24)
        } else {
            U128(amount)
        }
    }

    pub fn get_validator_info(&self, validator: AccountId) -> ValidatorInfoResponse {
        let validator_info = if let Some(val_info) = self.validator_info_map.get(&validator) {
            val_info
        } else {
            panic!("{}", errors::VALIDATOR_IS_NOT_PRESENT);
        };

        ValidatorInfoResponse {
            account_id: validator_info.account_id.clone(),
            staked: validator_info.staked.into(),
            unstaked: validator_info.unstaked.into(),
            last_asked_rewards_epoch_height: validator_info.last_redeemed_rewards_epoch.into(),
            available_for_unstake: validator_info.available_for_unstake.into(),
            lock: validator_info.lock,
        }
    }

    pub fn get_validators(&self) -> Vec<ValidatorInfoResponse> {
        self.validator_info_map
            .values()
            .map(|validator_info| ValidatorInfoResponse {
                account_id: validator_info.account_id.clone(),
                staked: validator_info.staked.into(),
                unstaked: validator_info.unstaked.into(),
                last_asked_rewards_epoch_height: validator_info.last_redeemed_rewards_epoch.into(),
                available_for_unstake: validator_info.available_for_unstake.into(),
                lock: validator_info.lock,
            })
            .collect()
    }

    pub fn validator_with_min_stake(&self) -> Option<ValidatorInfo> {
        self.validator_info_map
            .values()
            .filter(|v| v.unlocked())
            .min_by_key(|v| v.staked)
    }

    pub fn validator_available_for_unstake(&self) -> Option<ValidatorInfo> {
        self.validator_info_map
            .values()
            .filter(|v| v.unlocked() && v.available() && v.staked > MIN_STAKE_AMOUNT + ONE_NEAR)
            .max_by_key(|v| v.staked)
    }
}

/// Utility stuff.
#[near_bindgen]
impl NearxPool {
    /// Asserts that the method was called by the owner.
    pub fn assert_owner_calling(&self) {
        require!(
            env::predecessor_account_id() == self.owner_account_id,
            errors::ERROR_UNAUTHORIZED
        );
    }
    pub fn assert_operator_or_owner(&self) {
        require!(
            env::predecessor_account_id() == self.owner_account_id
                || env::predecessor_account_id() == self.operator_account_id,
            errors::ERROR_UNAUTHORIZED
        );
    }

    pub fn assert_not_busy(&self) {
        require!(!self.contract_lock, errors::ERROR_CONTRACT_BUSY);
    }

    pub fn assert_min_deposit_amount(&self, amount: u128) {
        require!(amount >= self.min_deposit_amount, errors::ERROR_MIN_DEPOSIT);
    }

    pub fn assert_staking_not_paused(&self) {
        require!(!self.staking_paused, errors::ERROR_STAKING_PAUSED);
    }

    pub fn get_current_epoch(&self) -> U64 {
        U64(env::epoch_height())
    }
}
