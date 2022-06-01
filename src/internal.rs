use crate::{
    constants::{gas, NO_DEPOSIT},
    utils::{amount_from_shares, assert_callback_calling, shares_from_amount},
    validator::*,
    *,
};
use near_sdk::{is_promise_success, log, AccountId, Balance, Promise, PromiseOrValue};

#[near_bindgen]
impl NearxPool {
    /// mints NearX based on user's deposited amount and current NearX price
    pub(crate) fn internal_deposit_and_stake(&mut self, user_amount: Balance) {
        log!("User deposited amount is {}", user_amount);
        self.assert_not_busy();

        self.assert_min_deposit_amount(user_amount);

        self.assert_staking_not_paused();

        let account_id = env::predecessor_account_id();

        // Calculate the number of nearx (stake shares) that the account will receive for staking the given amount.
        let num_shares = self.stake_shares_from_amount(user_amount);
        assert!(num_shares > 0);

        let sp_inx = self.get_stake_pool_with_min_stake();
        assert!(sp_inx.is_some(), "All pools busy");

        let sp_inx = sp_inx.unwrap();
        let selected_stake_pool = &mut self.validators[sp_inx];

        log!("Amount is {}", user_amount);
        //schedule async deposit_and_stake on that pool
        ext_staking_pool::ext(selected_stake_pool.account_id.clone())
            .with_static_gas(gas::DEPOSIT_AND_STAKE)
            .with_attached_deposit(user_amount)
            .deposit_and_stake()
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKING_POOL_DEPOSIT_AND_STAKE)
                    .on_stake_pool_deposit_and_stake(sp_inx, user_amount, num_shares, account_id),
            );
    }

    pub fn on_stake_pool_deposit_and_stake(
        &mut self,
        sp_inx: usize,
        amount: u128,
        shares: u128,
        user: AccountId,
    ) -> PromiseOrValue<bool> {
        assert_callback_calling();

        let sp = &mut self.validators[sp_inx];
        let stake_pool_account_id = sp.account_id.clone();
        let mut acc = &mut self.accounts.get(&user).unwrap_or_default();
        let mut transfer_funds = false;

        let stake_succeeded = is_promise_success();

        if stake_succeeded {
            //move into staked
            sp.staked += amount;
            acc.stake_shares += shares;
            self.total_stake_shares += shares;
            self.total_staked += amount;
            log!("Successfully staked {} into {}", amount, sp.account_id);
        } else {
            log!("Failed to stake {} into {}", amount, sp.account_id);
            transfer_funds = true;
            sp.lock = false;
            self.contract_lock = false;
        }

        if transfer_funds {
            log!(
                "Transferring back {} to {} after stake failed",
                amount,
                user
            );
            PromiseOrValue::Promise(Promise::new(user).transfer(amount))
        } else {
            log!("Reconciling total staked balance");
            self.internal_update_account(&user, acc);
            // Reconcile the total staked amount to the right value
            ext_staking_pool::ext(stake_pool_account_id)
                .with_static_gas(gas::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE)
                .with_attached_deposit(NO_DEPOSIT)
                .get_account_staked_balance(env::current_account_id())
                .then(
                    ext_staking_pool_callback::ext(env::current_account_id())
                        .with_attached_deposit(NO_DEPOSIT)
                        .with_static_gas(gas::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE)
                        .on_get_sp_staked_balance_reconcile(sp_inx, amount),
                );
            PromiseOrValue::Value(true)
        }
    }

    pub fn on_get_sp_staked_balance_reconcile(
        &mut self,
        sp_inx: usize,
        amount_actually_staked: u128,
        #[callback] total_staked_balance: U128String,
    ) {
        assert_callback_calling();

        self.contract_lock = false;
        let stake_pool = &mut self.validators[sp_inx];

        log!("Actual staked amount is {}", amount_actually_staked);

        // difference in staked amount and actual staked amount
        let difference_in_amount = stake_pool.staked.saturating_sub(total_staked_balance.0);
        // Reconcile the total staked with the actual total staked amount
        self.total_staked -= difference_in_amount;
        log!("Reconciled total staked to {}", self.total_staked);

        // Reconcile the stake pools total staked with the total staked balance
        stake_pool.staked = total_staked_balance.0;
        stake_pool.lock = false;
    }

    pub(crate) fn stake_shares_from_amount(&self, amount: Balance) -> u128 {
        shares_from_amount(amount, self.total_staked, self.total_stake_shares)
    }

    pub(crate) fn amount_from_stake_shares(&self, num_shares: u128) -> u128 {
        amount_from_shares(num_shares, self.total_staked, self.total_stake_shares)
    }

    pub(crate) fn internal_get_account(&self, account_id: &AccountId) -> Account {
        self.accounts.get(account_id).unwrap_or_default()
    }

    pub(crate) fn internal_update_account(&mut self, account_id: &AccountId, account: &Account) {
        if account.is_empty() {
            self.accounts.remove(account_id);
        } else {
            self.accounts.insert(account_id, account); //insert_or_update
        }
    }

    pub fn get_stake_pool_with_min_stake(&self) -> Option<usize> {
        let mut min_stake_amount: u128 = u128::MAX;
        let mut selected_sp_inx: Option<usize> = None;

        for (sp_inx, sp) in self.validators.iter().enumerate() {
            if !sp.lock && sp.staked < min_stake_amount {
                min_stake_amount = sp.staked;
                selected_sp_inx = Some(sp_inx);
            }
        }

        selected_sp_inx
    }
}
