use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::env::log;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, log, near_bindgen, require, serde_json, AccountId, PanicOnDefault, Promise, PromiseOrValue,
};

pub fn ntoy(near_amount: u128) -> u128 {
    near_amount * 10u128.pow(24)
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct HumanReadableAccount {
    pub account_id: AccountId,
    /// The unstaked balance that can be withdrawn or staked.
    pub unstaked_balance: U128,
    /// The amount balance staked at the current "stake" share price.
    pub staked_balance: U128,
    /// Whether the unstaked balance is available for withdrawal now.
    pub can_withdraw: bool,
}

/// Staking pool interface
trait StakingPool {
    fn get_account_staked_balance(&self, account_id: AccountId) -> U128;

    fn get_account_unstaked_balance(&self, account_id: AccountId) -> U128;

    fn get_account_total_balance(&self, account_id: AccountId) -> U128;

    fn get_account(&self, account_id: AccountId) -> HumanReadableAccount;

    fn deposit(&mut self);

    fn deposit_and_stake(&mut self);

    fn withdraw(&mut self, amount: U128);

    fn withdraw_all(&mut self);

    fn stake(&mut self, amount: U128);

    fn unstake(&mut self, amount: U128);

    fn unstake_all(&mut self);

    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

/// mockup of staking pool, for testing
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct MockStakingPool {
    deposits: LookupMap<AccountId, u128>,
    staked: LookupMap<AccountId, u128>,
    /// For ft_on_transfer testing
    refund_amount: U128,
    panic: bool,
}

#[near_bindgen]
impl MockStakingPool {
    #[init]
    pub fn new() -> Self {
        Self {
            deposits: LookupMap::new(b"d"),
            staked: LookupMap::new(b"s"),
            refund_amount: U128(ntoy(5)),
            panic: false,
        }
    }
}

#[near_bindgen]
impl StakingPool for MockStakingPool {
    fn get_account_staked_balance(&self, account_id: AccountId) -> U128 {
        U128::from(self.internal_get_staked(&account_id))
    }

    fn get_account_unstaked_balance(&self, account_id: AccountId) -> U128 {
        U128::from(self.internal_get_unstaked_deposit(&account_id))
    }

    fn get_account_total_balance(&self, account_id: AccountId) -> U128 {
        U128::from(
            self.internal_get_unstaked_deposit(&account_id) + self.internal_get_staked(&account_id),
        )
    }

    fn get_account(&self, account_id: AccountId) -> HumanReadableAccount {
        HumanReadableAccount {
            account_id: account_id.clone(),
            staked_balance: U128::from(self.internal_get_staked(&account_id)),
            unstaked_balance: U128::from(self.internal_get_unstaked_deposit(&account_id)),
            can_withdraw: true,
        }
    }

    #[payable]
    fn deposit(&mut self) {
        require!(!self.panic, "Test Panic!");
        self.internal_deposit()
    }

    #[payable]
    fn deposit_and_stake(&mut self) {
        require!(!self.panic, "Test Panic!");
        let account_id = env::predecessor_account_id();

        self.internal_deposit();

        // let amount = self.internal_get_unstaked_deposit(&account_id);
        self.internal_stake(env::attached_deposit());
    }

    fn withdraw(&mut self, amount: U128) {
        require!(!self.panic, "Test Panic!");
        let account_id = env::predecessor_account_id();
        self.internal_withdraw(&account_id, amount.0);
    }

    fn withdraw_all(&mut self) {
        require!(!self.panic, "Test Panic!");
        let account_id = env::predecessor_account_id();
        let unstaked = self.internal_get_unstaked_deposit(&account_id);
        self.internal_withdraw(&account_id, unstaked);
    }

    fn stake(&mut self, amount: U128) {
        require!(!self.panic, "Test Panic!");
        self.internal_stake(amount.0)
    }

    fn unstake(&mut self, amount: U128) {
        require!(!self.panic, "Test Panic!");
        self.internal_unstake(amount.0);
    }

    fn unstake_all(&mut self) {
        require!(!self.panic, "Test Panic!");
        let account_id = env::predecessor_account_id();
        let staked_amount = self.internal_get_staked(&account_id);
        self.internal_unstake(staked_amount);
    }

    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        require!(!self.panic);
        PromiseOrValue::Value(self.refund_amount)
    }
}

#[near_bindgen]
impl MockStakingPool {
    /// manually generate some reward for the caller,
    /// for testing purpose only
    pub fn add_reward(&mut self, amount: U128) {
        let account_id = env::predecessor_account_id();
        self.add_reward_for(amount, account_id);
    }

    pub fn set_refund_amount(&mut self, amount: U128) {
        self.refund_amount = amount;
    }

    pub fn add_reward_for(&mut self, amount: U128, account_id: AccountId) {
        let staked_amount = self.internal_get_staked(&account_id);
        assert!(staked_amount > 0);

        let new_amount = staked_amount + amount.0;
        self.staked.insert(&account_id, &new_amount);
    }

    pub fn set_panic(&mut self, panic: bool) {
        self.panic = panic;
    }

    pub fn adjust_balance(
        &mut self,
        account_id: AccountId,
        staked_delta: u128,
        unstaked_delta: u128,
    ) {
        let staked_amount = self
            .internal_get_staked(&account_id)
            .saturating_sub(staked_delta);
        let unstaked_amount = self.internal_get_unstaked_deposit(&account_id) + unstaked_delta;

        self.staked.insert(&account_id, &staked_amount);
        self.deposits.insert(&account_id, &unstaked_amount);
    }
}

impl MockStakingPool {
    fn internal_deposit(&mut self) {
        let account_id = env::predecessor_account_id();
        let amount = env::attached_deposit();
        assert!(amount > 0);

        let current_deposit = self.internal_get_unstaked_deposit(&account_id);
        let new_deposit = current_deposit + amount;

        self.deposits.insert(&account_id, &new_deposit);
    }

    fn internal_stake(&mut self, amount: u128) {
        let account_id = env::predecessor_account_id();
        let unstaked_deposit = self.internal_get_unstaked_deposit(&account_id);
        log!("amount is {}", amount);
        log!("unstaked deposit is {}", unstaked_deposit);
        assert!(unstaked_deposit >= amount);

        let new_deposit = unstaked_deposit - amount;
        let new_staked = self.internal_get_staked(&account_id) + amount;

        log!("New deposit amount is {}", new_deposit);
        log!("New staked amount is {}", new_staked);

        self.deposits.insert(&account_id, &new_deposit);
        self.staked.insert(&account_id, &new_staked);
    }

    fn internal_unstake(&mut self, amount: u128) {
        let account_id = env::predecessor_account_id();
        let staked = self.internal_get_staked(&account_id);
        assert!(staked >= amount);

        let unstaked_deposit = self.internal_get_unstaked_deposit(&account_id);
        let new_deposit = unstaked_deposit + amount;
        let new_staked = staked - amount;

        self.deposits.insert(&account_id, &new_deposit);
        self.staked.insert(&account_id, &new_staked);
    }

    fn internal_withdraw(&mut self, account_id: &AccountId, amount: u128) {
        let unstaked_amount = self.internal_get_unstaked_deposit(account_id);
        assert!(unstaked_amount >= amount);

        let new_unstaked = unstaked_amount - amount;
        self.deposits.insert(account_id, &new_unstaked);

        Promise::new(account_id.clone()).transfer(amount);
    }

    fn internal_get_unstaked_deposit(&self, account_id: &AccountId) -> u128 {
        self.deposits.get(account_id).unwrap_or_default()
    }

    fn internal_get_staked(&self, account_id: &AccountId) -> u128 {
        self.staked.get(account_id).unwrap_or_default()
    }
}
