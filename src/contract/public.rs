use crate::{
    constants::{NEAR, ONE_E24},
    contract::*,
    errors,
    state::*,
};
use near_sdk::near_bindgen;

#[near_bindgen]
impl NearxPool {
    #[init]
    pub fn new(owner_account_id: AccountId, operator_account_id: AccountId) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");

        Self {
            owner_account_id,
            contract_lock: false,
            operator_account_id,
            staking_paused: false,
            accumulated_staked_rewards: 0,
            total_stake_shares: 0,
            accounts: UnorderedMap::new(b"A".to_vec()),
            min_deposit_amount: NEAR,
            validators: Vec::new(),
            total_staked: 0,
            rewards_fee: Fraction::new(0, 1),
        }
    }

    /*
       Utility stuff
    */
    /// Asserts that the method was called by the owner.
    pub fn assert_owner_calling(&self) {
        assert_eq!(
            &env::predecessor_account_id(),
            &self.owner_account_id,
            "{}",
            errors::ERROR_UNAUTHORIZED
        )
    }
    pub fn assert_operator_or_owner(&self) {
        assert!(
            env::predecessor_account_id() == self.owner_account_id
                || env::predecessor_account_id() == self.operator_account_id,
            "{}",
            errors::ERROR_UNAUTHORIZED
        );
    }

    pub fn assert_not_busy(&self) {
        assert!(!self.contract_lock, "{}", errors::ERROR_CONTRACT_BUSY);
    }

    pub fn assert_min_deposit_amount(&self, amount: u128) {
        assert!(
            amount >= self.min_deposit_amount,
            "{}",
            errors::ERROR_MIN_DEPOSIT
        );
    }

    pub fn assert_staking_not_paused(&self) {
        assert!(!self.staking_paused, "{}", errors::ERROR_STAKING_PAUSED);
    }

    /*
       Main staking pool api
    */

    /// Rewards claiming
    pub fn ping(&mut self) {}

    #[payable]
    pub fn deposit(&mut self) {
        panic!("Use only deposit_and_stake");
    }

    /// Deposits the attached amount into the inner account of the predecessor and stakes it.
    #[payable]
    pub fn deposit_and_stake(&mut self) {
        self.internal_deposit_and_stake(env::attached_deposit());
    }

    /*
       Staking pool addition and deletion
    */
    pub fn remove_validator(&mut self, inx: u16) {
        self.assert_operator_or_owner();

        let sp = &self.validators[inx as usize];
        if !sp.is_empty() {
            panic!("sp is not empty")
        }
        self.validators.remove(inx as usize);
    }

    pub fn add_validator(&mut self, account_id: AccountId) {
        self.assert_operator_or_owner();
        for inx in 0..self.validators.len() {
            if self.validators[inx].account_id == account_id {
                panic!("already in list");
            }
        }
        self.validators.push(ValidatorInfo::new(account_id));
    }

    pub fn toggle_staking_pause(&mut self) {
        self.assert_operator_or_owner();
        self.staking_paused = !self.staking_paused;
    }

    // View methods

    pub fn get_account_staked_balance(&self, account_id: AccountId) -> U128String {
        self.get_account(account_id).staked_balance
    }

    pub fn get_account_total_balance(&self, account_id: AccountId) -> U128String {
        let acc = self.internal_get_account(&account_id);
        self.amount_from_stake_shares(acc.stake_shares).into()
    }

    pub fn is_account_unstaked_balance_available(&self, account_id: AccountId) -> bool {
        self.get_account(account_id).can_withdraw
    }

    pub fn get_owner_id(&self) -> AccountId {
        self.owner_account_id.clone()
    }

    pub fn get_reward_fee_fraction(&self) -> Fraction {
        self.rewards_fee
    }

    pub fn set_reward_fee(&mut self, numerator: u32, denominator: u32) {
        self.assert_owner_calling();
        assert!(numerator * 100 / denominator < 10); // less than 10%
        self.rewards_fee = Fraction::new(numerator, denominator);
    }

    pub fn get_total_staked(&self) -> U128String {
        U128String::from(self.total_staked)
    }

    pub fn get_staking_key(&self) -> PublicKey {
        panic!("No staking key for the staking pool");
    }

    pub fn is_staking_paused(&self) -> bool {
        self.staking_paused
    }

    pub fn get_account(&self, account_id: AccountId) -> AccountResponse {
        let account = self.internal_get_account(&account_id);
        println!("account is {:?}", account);
        AccountResponse {
            account_id,
            unstaked_balance: U128String::from(0), // TODO - implement unstake
            staked_balance: self.amount_from_stake_shares(account.stake_shares).into(),
            can_withdraw: false,
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
            total_staked: U128String::from(self.total_staked),
            total_stake_shares: U128String::from(self.total_stake_shares),
            accumulated_staked_rewards: U128String::from(self.accumulated_staked_rewards),
            min_deposit_amount: U128String::from(self.min_deposit_amount),
            operator_account_id: self.operator_account_id.clone(),
            rewards_fee_pct: self.rewards_fee.clone(),
        }
    }

    pub fn get_nearx_price(&self) -> U128String {
        self.amount_from_stake_shares(ONE_E24).into()
    }

    // Staking pool query
    pub fn get_validator_info(&self, inx: u16) -> ValidatorInfoResponse {
        assert!((inx as usize) < self.validators.len());
        let sp = &self.validators[inx as usize];

        ValidatorInfoResponse {
            inx,
            account_id: sp.account_id.clone(),
            staked: sp.staked.into(),
            last_asked_rewards_epoch_height: sp.last_redeemed_rewards_epoch.into(),
            lock: sp.lock,
        }
    }

    pub fn get_validators(&self) -> Vec<ValidatorInfoResponse> {
        self.validators
            .iter()
            .enumerate()
            .map(|(i, pool)| ValidatorInfoResponse {
                inx: i as u16,
                account_id: pool.account_id.clone(),
                staked: U128String::from(pool.staked),
                last_asked_rewards_epoch_height: U64String::from(pool.last_redeemed_rewards_epoch),
                lock: pool.lock,
            })
            .collect()
    }
}
