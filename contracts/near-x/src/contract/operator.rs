use crate::{
    constants::{gas, NO_DEPOSIT},
    contract::*,
    errors::ERROR_VALIDATOR_IS_BUSY,
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
}
