use crate::constants::NO_DEPOSIT;
use crate::staking_pool::*;
use crate::utils::{apply_multiplier, assert_callback_calling};
use crate::*;
use near_sdk::{log, near_bindgen};

#[near_bindgen]
impl NearxPool {
    // This method queries the stake pol contract to check if there are any rewards to account for
    // if there are rewards since the last call of distribute_rewards, we increase the total_staked
    // amount which will increase the price of nearX
    pub fn distribute_rewards(&mut self, sp_inx: u16) {
        self.assert_not_busy();

        let inx = sp_inx as usize;
        assert!(inx < self.staking_pools.len());

        let sp = &mut self.staking_pools[inx];
        assert!(!sp.lock, "sp is busy");

        let epoch_height = env::epoch_height();

        if sp.staked == 0 {
            return;
        }

        if sp.last_asked_rewards_epoch_height == epoch_height {
            return;
        }

        log!(
            "Fetching total balance from the staking pool {}",
            sp.account_id
        );

        self.contract_lock = true;
        sp.lock = true;

        //query our current balance (includes staked+unstaked+staking rewards)
        ext_staking_pool::get_account_staked_balance(
            env::current_account_id(),
            //promise params
            &sp.account_id,
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
        sp_inx: usize,
        #[callback] total_staked_balance: U128String,
    ) {
        assert_callback_calling();

        let sp = &mut self.staking_pools[sp_inx];

        sp.lock = false;
        self.contract_lock = false;

        sp.last_asked_rewards_epoch_height = env::epoch_height();

        //new_total_balance has the new staked amount for this pool
        let new_total_balance = total_staked_balance.0;
        log!("total staked balance is {}", total_staked_balance.0);

        let rewards = if new_total_balance < sp.total_balance() {
            log!(
                "INCONSISTENCY @{} says new_total_balance < our info sp.total_balance()",
                sp.account_id
            );
            0
        } else {
            //compute rewards, as new balance minus old balance
            new_total_balance - sp.total_balance()
        };

        log!(
            "sp:{} old_balance:{} new_balance:{} rewards:{}",
            sp.account_id,
            sp.total_balance(),
            new_total_balance,
            rewards
        );

        //updated accumulated_staked_rewards value for the contract
        self.accumulated_staked_rewards += rewards;
        //updated new "staked" value for this pool
        sp.staked = new_total_balance;

        if rewards > 0 {
            self.total_staked += rewards;

            // compute the reward fee
            let mut operator_account = self.internal_get_account(&self.operator_account_id.clone());
            println!("operator_account is {:?}", self.operator_account_id.clone());
            let operator_fee = apply_multiplier(rewards, self.rewards_fee_pct);
            println!("operator_fee is {:?}", operator_fee);
            let operator_fee_shares = self.stake_shares_from_amount(operator_fee);
            println!("operator_shares is {:?}", operator_fee_shares);
            if operator_fee_shares > 0 {
                operator_account.stake_shares += operator_fee_shares;
                self.internal_update_account(&self.operator_account_id.clone(), &operator_account);
            }
        }
    }
}
