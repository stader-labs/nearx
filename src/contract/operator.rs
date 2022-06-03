use crate::{
    constants::{gas, NO_DEPOSIT},
    contract::*,
    errors::ERROR_VALIDATOR_IS_BUSY,
    state::*,
    utils::assert_callback_calling,
};
use near_sdk::{log, near_bindgen};

#[near_bindgen]
impl NearxPool {
    pub fn autocompound_rewards(&mut self, validator: AccountId) {
        self.assert_not_busy();

        let mut validator_info = self.internal_get_validator(&validator);

        assert!(!validator_info.lock, "{}", ERROR_VALIDATOR_IS_BUSY);

        let epoch_height = env::epoch_height();

        println!("validator staked amount is {:?}", validator_info.staked);
        if validator_info.staked == 0 {
            return;
        }

        if validator_info.last_redeemed_rewards_epoch == epoch_height {
            return;
        }

        log!(
            "Fetching total balance from the staking pool {}",
            validator_info.account_id
        );

        self.contract_lock = true;
        validator_info.lock = true;

        self.internal_update_validator(&validator_info);
        println!("setting validator lock to true");

        ext_staking_pool::ext(validator_info.account_id.clone())
            .with_attached_deposit(NO_DEPOSIT)
            .with_static_gas(gas::GET_ACCOUNT_TOTAL_BALANCE)
            .get_account_staked_balance(env::current_account_id())
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE)
                    .on_get_sp_staked_balance_for_rewards(validator_info),
            );
    }

    pub fn on_get_sp_staked_balance_for_rewards(
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
            PromiseOrValue::Promise(Promise::new(env::current_account_id()).transfer(operator_fee))
        } else {
            PromiseOrValue::Value(true)
        }
    }
}
