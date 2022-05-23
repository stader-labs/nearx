use crate::constants::NO_DEPOSIT;
use crate::staking_pool::*;
use crate::utils::{amount_from_shares, assert_callback_calling, shares_from_amount};
use crate::*;
use near_sdk::{
    is_promise_success,
    json_types::{ValidAccountId, U128},
    log, AccountId, Balance, Promise, PromiseOrValue, PromiseResult,
};

#[near_bindgen]
impl NearxPool {
    pub(crate) fn native_transfer_to_predecessor(&mut self, amount: u128) -> Promise {
        self.contract_account_balance -= amount;
        return Promise::new(env::predecessor_account_id()).transfer(amount);
    }

    /// mints NearX based on user's deposited amount and current NearX price
    pub(crate) fn internal_deposit_and_stake(&mut self, user_amount: Balance) {
        log!("User deposited amount is {}", user_amount);
        self.assert_not_busy();

        self.assert_min_deposit_amount(user_amount);

        self.assert_staking_not_paused();

        let account_id = env::predecessor_account_id();

        self.contract_account_balance += user_amount;

        // Calculate the number of nearx (stake shares) that the account will receive for staking the given amount.
        let num_shares = self.stake_shares_from_amount(user_amount);
        assert!(num_shares > 0);

        let (sp_inx, amount) = self.get_stake_pool_with_min_stake();
        assert!(sp_inx.is_some(), "All pools busy");

        let sp_inx = sp_inx.unwrap();
        let selected_stake_pool = &mut self.staking_pools[sp_inx];

        log!("Amount is {}", user_amount);
        //schedule async deposit_and_stake on that pool
        ext_staking_pool::deposit_and_stake(
            &selected_stake_pool.account_id,
            user_amount, //attached amount
            // promise parameters
            gas::staking_pool::DEPOSIT_AND_STAKE,
        )
        .then(ext_staking_pool_callback::on_stake_pool_deposit_and_stake(
            sp_inx,
            user_amount,
            num_shares,
            account_id.clone(),
            // promise parameters
            &env::current_account_id(),
            NO_DEPOSIT,
            gas::owner_callbacks::ON_STAKING_POOL_DEPOSIT_AND_STAKE,
        ));
    }

    pub fn on_stake_pool_deposit_and_stake(
        &mut self,
        sp_inx: usize,
        amount: u128,
        shares: u128,
        user: AccountId,
    ) -> PromiseOrValue<bool> {
        assert_callback_calling();

        let sp = &mut self.staking_pools[sp_inx];
        let stake_pool_account_id = sp.account_id.clone();
        let mut acc = &mut self.accounts.get(&user).unwrap_or_default();
        let mut transfer_funds = false;

        let stake_succeeded = is_promise_success();

        if stake_succeeded {
            //we send NEAR to the staking-pool
            //we took from contract balance (transfer)
            self.contract_account_balance -= amount;
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

        return if transfer_funds {
            log!(
                "Transferring back {} to {} after stake failed",
                amount,
                user
            );
            PromiseOrValue::Promise(Promise::new(user).transfer(amount))
        } else {
            log!("Reconciling total staked balance");
            self.internal_update_account(&user, &acc);
            // Reconcile the total staked amount to the right value
            ext_staking_pool::get_account_staked_balance(
                env::current_account_id(),
                // promise params
                &stake_pool_account_id,
                NO_DEPOSIT,
                gas::owner_callbacks::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE,
            )
            .then(
                ext_staking_pool_callback::on_get_sp_staked_balance_reconcile(
                    sp_inx,
                    amount,
                    // promise params
                    &env::current_account_id(),
                    NO_DEPOSIT,
                    gas::owner_callbacks::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE,
                ),
            );
            PromiseOrValue::Value(true)
        };
    }

    pub fn on_get_sp_staked_balance_reconcile(
        &mut self,
        sp_inx: usize,
        amount_actually_staked: u128,
        #[callback] total_staked_balance: U128String,
    ) {
        assert_callback_calling();

        self.contract_lock = false;
        let stake_pool = &mut self.staking_pools[sp_inx];

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

    /// Returns the number of NearX (stake shares) corresponding to the given near amount at current NearX price
    /// if the amount & the shares are incorporated, price remains the same
    pub(crate) fn stake_shares_from_amount(&self, amount: Balance) -> u128 {
        return shares_from_amount(amount, self.total_staked, self.total_stake_shares);
    }

    /// Returns the amount corresponding to the given number of NearX (stake shares).
    pub(crate) fn amount_from_stake_shares(&self, num_shares: u128) -> u128 {
        return amount_from_shares(num_shares, self.total_staked, self.total_stake_shares);
    }

    /// Inner method to get the given account or a new default value account.
    pub(crate) fn internal_get_account(&self, account_id: &AccountId) -> Account {
        self.accounts.get(account_id).unwrap_or_default()
    }

    /// Inner method to save the given account for a given account ID.
    /// If the account balances are 0, the account is deleted instead to release storage.
    pub(crate) fn internal_update_account(&mut self, account_id: &AccountId, account: &Account) {
        if account.is_empty() {
            self.accounts.remove(account_id);
        } else {
            self.accounts.insert(account_id, &account); //insert_or_update
        }
    }

    /// Get the stake pool with the minimum stake
    pub(crate) fn get_stake_pool_with_min_stake(&self) -> (Option<usize>, u128) {
        let mut min_stake_amount: u128 = u128::MAX;
        let mut selected_sp_inx: Option<usize> = None;

        for (sp_inx, sp) in self.staking_pools.iter().enumerate() {
            // if the pool is not busy, and this pool can stake
            if !sp.lock {
                // this pool requires staking?
                if sp.staked < min_stake_amount {
                    min_stake_amount = sp.staked;
                    selected_sp_inx = Some(sp_inx);
                }
            }
        }

        return (selected_sp_inx, min_stake_amount);
    }

    pub fn internal_nearx_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
    ) {
        assert_ne!(
            sender_id, receiver_id,
            "Sender and receiver should be different"
        );
        assert!(amount > 0, "The amount should be a positive number");
        let mut sender_acc = self.internal_get_account(&sender_id);
        let mut receiver_acc = self.internal_get_account(&receiver_id);
        assert!(
            amount <= sender_acc.stake_shares,
            "{} does not have enough NearX balance {}",
            sender_id,
            sender_acc.stake_shares
        );

        sender_acc.sub_stake_shares(amount);
        receiver_acc.add_stake_shares(amount);

        self.internal_update_account(&sender_id, &sender_acc);
        self.internal_update_account(&receiver_id, &receiver_acc);
    }

    pub fn int_ft_resolve_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> (u128, u128) {
        let sender_id: AccountId = sender_id.into();
        let receiver_id: AccountId = receiver_id.into();
        let amount: Balance = amount.into();

        // Get the unused amount from the `ft_on_transfer` call result.
        let unused_amount = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount, unused_amount.0)
                } else {
                    amount
                }
            }
            PromiseResult::Failed => amount,
        };

        if unused_amount > 0 {
            let mut receiver_acc = self.internal_get_account(&receiver_id);
            let receiver_balance = receiver_acc.stake_shares;
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                receiver_acc.sub_stake_shares(refund_amount);
                self.internal_update_account(&receiver_id, &receiver_acc);

                let mut sender_acc = self.internal_get_account(&sender_id);
                sender_acc.add_stake_shares(refund_amount);
                self.internal_update_account(&sender_id, &sender_acc);

                log!(
                    "Refund {} from {} to {}",
                    refund_amount,
                    receiver_id,
                    sender_id
                );
                return (amount - refund_amount, 0);
            }
        }
        (amount, 0)
    }
}
