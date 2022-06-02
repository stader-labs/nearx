use crate::errors::{
    ERROR_CONTRACT_ALREADY_INITIALIZED, ERROR_NO_STAKING_KEY, ERROR_VALIDATOR_IS_ALREADY_PRESENT,
    ERROR_VALIDATOR_IS_NOT_PRESENT,
};
use crate::{
    constants::{NEAR, ONE_E24},
    contract::*,
    errors,
    state::*,
};
use near_sdk::json_types::U64;
use near_sdk::log;
use near_sdk::near_bindgen;

#[near_bindgen]
impl NearxPool {
    #[init]
    pub fn new(owner_account_id: AccountId, operator_account_id: AccountId) -> Self {
        assert!(
            !env::state_exists(),
            "{}",
            ERROR_CONTRACT_ALREADY_INITIALIZED
        );

        Self {
            owner_account_id,
            contract_lock: false,
            operator_account_id,
            staking_paused: false,
            to_withdraw: 0,
            accumulated_staked_rewards: 0,
            total_stake_shares: 0,
            accounts: UnorderedMap::new(b"A".to_vec()),
            min_deposit_amount: NEAR,
            validator_info_map: UnorderedMap::new(b"B".to_vec()),
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
        unimplemented!();
    }

    /// Deposits the attached amount into the inner account of the predecessor and stakes it.
    #[payable]
    pub fn deposit_and_stake(&mut self) {
        self.internal_deposit_and_stake(env::attached_deposit());
    }

    /*
       Staking pool addition and deletion
    */
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

    // View methods

    pub fn get_account_staked_balance(&self, account_id: AccountId) -> U128 {
        self.get_account(account_id).staked_balance
    }

    pub fn get_account_total_balance(&self, account_id: AccountId) -> U128 {
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
        assert!((numerator * 100 / denominator) < 10); // less than 10%
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
        println!("account is {:?}", account);
        AccountResponse {
            account_id,
            unstaked_balance: U128::from(0), // TODO - implement unstake//
            staked_balance: self.amount_from_stake_shares(account.stake_shares).into(),
            stake_shares: account.stake_shares.into(),
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
            total_staked: U128::from(self.total_staked),
            total_stake_shares: U128::from(self.total_stake_shares),
            accumulated_staked_rewards: U128::from(self.accumulated_staked_rewards),
            min_deposit_amount: U128::from(self.min_deposit_amount),
            operator_account_id: self.operator_account_id.clone(),
            rewards_fee_pct: self.rewards_fee,
        }
    }

    pub fn get_nearx_price(&self) -> U128 {
        self.amount_from_stake_shares(ONE_E24).into()
    }

    pub fn get_validator_info(&self, validator: AccountId) -> ValidatorInfoResponse {
        let validator_info = if let Some(val_info) = self.validator_info_map.get(&validator) {
            val_info
        } else {
            panic!("{}", ERROR_VALIDATOR_IS_NOT_PRESENT);
        };

        ValidatorInfoResponse {
            account_id: validator_info.account_id.clone(),
            staked: validator_info.staked.into(),
            last_asked_rewards_epoch_height: validator_info.last_redeemed_rewards_epoch.into(),
            lock: validator_info.lock,
        }
    }

    pub fn get_validators(&self) -> Vec<ValidatorInfoResponse> {
        self.validator_info_map
            .iter()
            .map(|pool| ValidatorInfoResponse {
                account_id: pool.1.account_id.clone(),
                staked: U128::from(pool.1.staked),
                last_asked_rewards_epoch_height: U64(pool.1.last_redeemed_rewards_epoch),
                lock: pool.1.lock,
            })
            .collect()
    }
}
