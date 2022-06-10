use crate::{
    constants::{gas, NO_DEPOSIT},
    contract::*,
    state::ValidatorInfo,
    utils::assert_callback_calling,
};
use near_sdk::{is_promise_success, log, AccountId, PromiseOrValue};

#[near_bindgen]
impl ExtNearxStakingPoolCallbacks for NearxPool {
    #[private]
    fn on_stake_pool_deposit(&mut self, amount: U128) -> bool {
        let _ = amount;
        todo!()
    }

    #[private]
    fn on_stake_pool_deposit_and_stake(&mut self, validator: AccountId, amount: Balance) {
        let mut validator_info = self.internal_get_validator(&validator);
        if is_promise_success() {
            validator_info.staked += amount;
            // reconcile total staked amount to the actual total staked amount
        } else {
            self.user_amount_to_stake_in_epoch += amount;
        }

        self.internal_update_validator(&validator_info);
    }

    #[private]
    fn on_stake_pool_deposit_and_stake_direct(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
        amount: u128,
        shares: u128,
        user: AccountId,
    ) -> PromiseOrValue<bool> {
        assert_callback_calling();

        let mut acc = &mut self.accounts.get(&user).unwrap_or_default();
        let mut transfer_funds = false;

        let stake_succeeded = is_promise_success();
        println!("stake_succeeded {:?}", stake_succeeded);

        if stake_succeeded {
            validator_info.staked += amount;
            acc.stake_shares += shares;
            self.total_stake_shares += shares;
            self.total_staked += amount;
            log!(
                "Successfully staked {} into {}",
                amount,
                validator_info.account_id
            );
        } else {
            log!(
                "Failed to stake {} into {}",
                amount,
                validator_info.account_id
            );
            transfer_funds = true;
            validator_info.lock = false;
            self.contract_lock = false;
        }

        self.internal_update_validator(&validator_info);

        if transfer_funds {
            log!("Transfering back {} to {} after stake failed", amount, user);
            PromiseOrValue::Promise(Promise::new(user).transfer(amount))
        } else {
            log!("Reconciling total staked balance");
            self.internal_update_account(&user, acc);
            // Reconcile the total staked amount to the right value
            ext_staking_pool::ext(validator_info.account_id.clone())
                .with_static_gas(gas::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE)
                .with_attached_deposit(NO_DEPOSIT)
                .get_account_staked_balance(env::current_account_id())
                .then(
                    ext_staking_pool_callback::ext(env::current_account_id())
                        .with_attached_deposit(NO_DEPOSIT)
                        .with_static_gas(gas::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE)
                        .on_get_sp_staked_balance_reconcile(validator_info, amount),
                );
            PromiseOrValue::Value(true)
        }
    }

    #[private]
    fn on_stake_pool_epoch_unstake(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
        near_amount: Balance,
    ) -> PromiseOrValue<bool> {
        assert_callback_calling();

        let unstake_succeeded = is_promise_success();
        println!("unstake_succeeded {:?}", unstake_succeeded);

        if unstake_succeeded {
            log!(
                "Successfully unstaked {} from {}",
                near_amount,
                validator_info.account_id,
            );

            validator_info.to_withdraw += near_amount;
        } else {
            log!(
                "Failed to unstake {} from {}",
                near_amount,
                validator_info.account_id,
            );

            // ROLLBACK:
            // Validator update:
            validator_info.staked += near_amount;
            // Pool update:
            //self.total_staked += near_amount;
            self.to_unstake += near_amount;
            self.to_withdraw -= near_amount;

            validator_info.lock = false;
            self.contract_lock = false;
        }

        self.internal_update_validator(&validator_info);

        PromiseOrValue::Value(true)
    }

    #[private]
    fn on_stake_pool_epoch_withdraw(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
    ) -> PromiseOrValue<bool> {
        assert_callback_calling();

        let withdraw_succeeded = is_promise_success();
        println!("withdraw_succeeded {:?}", withdraw_succeeded);

        validator_info.to_withdraw = 0;
        self.internal_update_validator(&validator_info);

        PromiseOrValue::Value(withdraw_succeeded)
    }

    #[private]
    fn on_get_sp_total_balance(
        &mut self,
        validator_info: ValidatorInfo,
        #[callback] total_balance: U128,
    ) {
        let _ = (validator_info, total_balance);
        todo!()
    }

    #[private]
    fn on_get_sp_staked_balance_for_rewards(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
        #[callback] total_staked_balance: U128,
    ) -> PromiseOrValue<bool> {
        assert_callback_calling();

        validator_info.lock = false;
        self.contract_lock = false;

        validator_info.last_redeemed_rewards_epoch = env::epoch_height();

        //new_total_balance has the new staked amount for this pool
        let new_total_balance = total_staked_balance.0;
        log!("total staked balance is {}", total_staked_balance.0);

        //compute rewards, as new balance minus old balance
        let rewards = new_total_balance.saturating_sub(validator_info.total_balance());

        log!(
            "validator account:{} old_balance:{} new_balance:{} rewards:{}",
            validator_info.account_id,
            validator_info.total_balance(),
            new_total_balance,
            rewards
        );

        //updated accumulated_staked_rewards value for the contract
        self.accumulated_staked_rewards += rewards;
        //updated new "staked" value for this pool
        validator_info.staked = new_total_balance;

        let operator_fee = rewards * self.rewards_fee;
        self.total_staked += rewards;

        self.internal_update_validator(&validator_info);

        if operator_fee > 0 {
            PromiseOrValue::Promise(
                Promise::new(self.operator_account_id.clone()).transfer(operator_fee),
            )
        } else {
            PromiseOrValue::Value(true)
        }
    }

    #[private]
    fn on_get_sp_staked_balance_reconcile(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
        amount_actually_staked: u128,
        #[callback] total_staked_balance: U128,
    ) {
        assert_callback_calling();

        self.contract_lock = false;

        log!("Actual staked amount is {}", amount_actually_staked);

        // difference in staked amount and actual staked amount
        let difference_in_amount = validator_info.staked.saturating_sub(total_staked_balance.0);
        // Reconcile the total staked with the actual total staked amount
        self.total_staked -= difference_in_amount;
        log!("Reconciled total staked to {}", self.total_staked);

        // Reconcile the stake pools total staked with the total staked balance
        validator_info.staked = total_staked_balance.0;
        validator_info.lock = false;

        self.internal_update_validator(&validator_info);
    }

    #[private]
    fn on_get_sp_unstaked_balance(
        &mut self,
        validator_info: ValidatorInfo,
        #[callback] unstaked_balance: U128,
    ) {
        let _ = (validator_info, unstaked_balance);
        todo!()
    }
}
