use crate::constants::NO_DEPOSIT;
use crate::errors::ERROR_VALIDATOR_IS_BUSY;
use crate::utils::{apply_multiplier, assert_callback_calling};
use crate::validator::*;
use crate::*;
use near_sdk::{log, near_bindgen, Promise, PromiseOrValue};

#[near_bindgen]
impl NearxPool {
    // This method queries the stake pol contract to check if there are any rewards to account for
    // if there are rewards since the last call of distribute_rewards, we increase the total_staked
    // amount which will increase the price of nearX
    pub fn distribute_rewards(&mut self, val_inx: u16) {
        self.assert_not_busy();

        let inx = val_inx as usize;
        assert!(inx < self.validators.len());

        let val = &mut self.validators[inx];
        assert!(!val.lock, ERROR_VALIDATOR_IS_BUSY);

        let epoch_height = env::epoch_height();

        if val.staked == 0 {
            return;
        }

        if val.last_asked_rewards_epoch_height == epoch_height {
            return;
        }

        log!(
            "Fetching total balance from the staking pool {}",
            val.account_id
        );

        self.contract_lock = true;
        val.lock = true;

        //query our current balance (includes staked+unstaked+staking rewards)
        ext_staking_pool::get_account_staked_balance(
            env::current_account_id(),
            //promise params
            &val.account_id,
            NO_DEPOSIT,
            gas::staking_pool::GET_ACCOUNT_TOTAL_BALANCE,
        )
        .then(
            ext_staking_pool_callback::on_get_sp_staked_balance_for_rewards(
                inx,
                //promise params
                &env::current_account_id(),
                NO_DEPOSIT,
                gas::owner_callbacks::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE,
            ),
        );
    }

    pub fn on_get_sp_staked_balance_for_rewards(
        &mut self,
        val_inx: usize,
        #[callback] total_staked_balance: U128String,
    ) {
        assert_callback_calling();

        //new_total_balance has the new staked amount for this pool
        let new_total_balance: u128;
        let val = &mut self.validators[val_inx];

        val.lock = false;
        self.contract_lock = false;

        val.last_asked_rewards_epoch_height = env::epoch_height();

        new_total_balance = total_staked_balance.0;
        log!("total staked balance is {}", total_staked_balance.0);

        //compute rewards, as new balance minus old balance
        let rewards = new_total_balance.saturating_sub(val.total_balance());

        log!(
            "sp:{} old_balance:{} new_balance:{} rewards:{}",
            val.account_id,
            val.total_balance(),
            new_total_balance,
            rewards
        );

        //updated accumulated_staked_rewards value for the contract
        self.accumulated_staked_rewards += rewards;
        //updated new "staked" value for this pool
        val.staked = new_total_balance;

        if rewards > 0 {
            self.total_staked += rewards;

            // compute the reward fee
            let mut operator_account = self.internal_get_account(&self.operator_account_id.clone());
            let operator_fee = apply_multiplier(rewards, self.rewards_fee_pct);
            let operator_fee_shares = self.stake_shares_from_amount(operator_fee);
            if operator_fee_shares > 0 {
                operator_account.stake_shares += operator_fee_shares;
                self.internal_update_account(&self.operator_account_id.clone(), &operator_account);
            }
        }
    }
}
