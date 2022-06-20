use crate::constants::{ACCOUNTS_MAP, VALIDATOR_MAP};
use crate::errors::*;
use crate::events::Event;
use crate::{contract::*, errors, state::*};
use near_sdk::json_types::U64;
use near_sdk::near_bindgen;
use near_sdk::{assert_one_yocto, log, require, ONE_NEAR};

#[near_bindgen]
impl NearxPool {
    #[init]
    pub fn new(owner_account_id: AccountId, operator_account_id: AccountId) -> Self {
        require!(!env::state_exists(), ERROR_CONTRACT_ALREADY_INITIALIZED);

        Self {
            owner_account_id,
            contract_lock: false,
            operator_account_id,
            accumulated_staked_rewards: 0,
            user_amount_to_stake_in_epoch: 0,
            user_amount_to_unstake_in_epoch: 0,
            reconciled_epoch_stake_amount: 0,
            reconciled_epoch_unstake_amount: 0,
            total_stake_shares: 0,
            accounts: UnorderedMap::new(ACCOUNTS_MAP.as_bytes()),
            min_deposit_amount: ONE_NEAR,
            validator_info_map: UnorderedMap::new(VALIDATOR_MAP.as_bytes()),
            total_staked: 0,
            rewards_fee: Fraction::new(0, 1),
            last_reconcilation_epoch: 0,
            temp_owner: None,
            operations_control: OperationControls {
                stake_paused: false,
                unstaked_paused: false,
                withdraw_paused: false,
                epoch_stake_paused: false,
                epoch_unstake_paused: false,
                epoch_withdraw_paused: false,
                epoch_autocompounding_paused: false,
            },
        }
    }

    /*
       Utility stuff
    */
    /// Asserts that the method was called by the owner.
    pub fn assert_owner_calling(&self) {
        require!(
            env::predecessor_account_id() == self.owner_account_id,
            errors::ERROR_UNAUTHORIZED
        )
    }
    pub fn assert_operator_or_owner(&self) {
        require!(
            env::predecessor_account_id() == self.owner_account_id
                || env::predecessor_account_id() == self.operator_account_id,
            errors::ERROR_UNAUTHORIZED
        );
    }

    pub fn assert_not_busy(&self) {
        require!(!self.contract_lock, errors::ERROR_CONTRACT_BUSY);
    }

    pub fn assert_min_deposit_amount(&self, amount: u128) {
        require!(amount >= self.min_deposit_amount, errors::ERROR_MIN_DEPOSIT);
    }

    pub fn assert_staking_not_paused(&self) {
        require!(
            !self.operations_control.stake_paused,
            errors::ERROR_STAKING_PAUSED
        );
    }

    pub fn assert_unstaking_not_paused(&self) {
        require!(
            !self.operations_control.unstaked_paused,
            errors::ERROR_UNSTAKING_PAUSED
        );
    }

    pub fn assert_withdraw_not_paused(&self) {
        require!(
            !self.operations_control.withdraw_paused,
            errors::ERROR_UNSTAKING_PAUSED
        );
    }

    pub fn assert_epoch_stake_not_paused(&self) {
        require!(
            !self.operations_control.epoch_stake_paused,
            errors::ERROR_EPOCH_STAKE_PAUSED
        );
    }

    pub fn assert_epoch_unstake_not_paused(&self) {
        require!(
            !self.operations_control.epoch_unstake_paused,
            errors::ERROR_EPOCH_UNSTAKE_PAUSED
        );
    }

    pub fn assert_epoch_withdraw_not_paused(&self) {
        require!(
            !self.operations_control.epoch_withdraw_paused,
            errors::ERROR_EPOCH_WITHDRAW_PAUSED
        );
    }

    pub fn assert_epoch_autocompounding_not_paused(&self) {
        require!(
            !self.operations_control.epoch_autocompounding_paused,
            errors::ERROR_EPOCH_AUTOCOMPOUNDING_PAUSED
        );
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
    pub fn deposit_and_stake_direct_stake(&mut self) {
        self.internal_deposit_and_stake_direct_stake(env::attached_deposit());
    }

    #[payable]
    pub fn deposit_and_stake(&mut self) {
        self.internal_deposit_and_stake(env::attached_deposit());
    }

    /// Unstakes all staked balance from the inner account of the predecessor.
    /// The new total unstaked balance will be available for withdrawal in four epochs.
    pub fn unstake_all(&mut self) {
        let account_id = env::predecessor_account_id();
        let account = self.internal_get_account(&account_id);
        let amount = self.staked_amount_from_num_shares_rounded_down(account.stake_shares);
        self.internal_unstake(amount);
    }

    /// Unstakes the given amount from the inner account of the predecessor.
    /// The inner account should have enough staked balance.
    /// The new total unstaked balance will be available for withdrawal in four epochs.
    pub fn unstake(&mut self, amount: U128) {
        let amount: Balance = amount.into();
        self.internal_unstake(amount);
    }

    /// Withdraws the entire unstaked balance from the predecessor account.
    /// It's only allowed if the `unstake` action was not performed in the four most recent epochs.
    pub fn withdraw_all(&mut self) {
        let account_id = env::predecessor_account_id();
        let account = self.internal_get_account(&account_id);
        self.internal_withdraw(account.unstaked_amount);
    }

    /// Withdraws the non staked balance for given account.
    /// It's only allowed if the `unstake` action was not performed in the four most recent epochs.
    pub fn withdraw(&mut self, amount: U128) {
        let amount: Balance = amount.into();
        self.internal_withdraw(amount);
    }

    /*
       Staking pool addition and deletion
    */
    pub fn pause_validator(&mut self, validator: AccountId) {
        self.assert_owner_calling();

        let mut validator_info = self.internal_get_validator(&validator);

        require!(
            !validator_info.pending_unstake_release(),
            ERROR_VALIDATOR_UNSTAKE_STILL_UNBONDING
        );

        validator_info.paused = true;
        self.internal_update_validator(&validator, &validator_info);
    }

    pub fn un_pause_validator(&mut self, validator: AccountId) {
        self.assert_owner_calling();

        let mut validator_info = self.internal_get_validator(&validator);
        validator_info.paused = false;
        self.internal_update_validator(&validator, &validator_info);
    }

    pub fn remove_validator(&mut self, validator: AccountId) {
        self.assert_operator_or_owner();

        let validator_info = self.internal_get_validator(&validator);

        log!("validator is paused {:?}", validator_info.paused());
        log!(
            "validator is unbonding {:?}",
            validator_info.pending_unstake_release()
        );

        require!(validator_info.is_empty(), ERROR_INVALID_VALIDATOR_REMOVAL);

        self.validator_info_map.remove(&validator);

        Event::ValidatorRemoved {
            account_id: validator,
        }
        .emit();
    }

    pub fn add_validator(&mut self, validator: AccountId) {
        self.assert_operator_or_owner();
        if self.validator_info_map.get(&validator).is_some() {
            panic!("{}", ERROR_VALIDATOR_IS_ALREADY_PRESENT);
        }
        self.validator_info_map
            .insert(&validator, &ValidatorInfo::new(validator.clone()));

        Event::ValidatorAdded {
            account_id: validator,
        }
        .emit();
    }

    pub fn toggle_staking_pause(&mut self) {
        self.assert_operator_or_owner();
        self.operations_control.stake_paused = !self.operations_control.stake_paused;
    }

    // Owner update methods
    #[payable]
    pub fn set_owner(&mut self, new_owner: AccountId) {
        assert_one_yocto();
        require!(
            env::predecessor_account_id() == self.owner_account_id,
            ERROR_UNAUTHORIZED
        );
        self.temp_owner = Some(new_owner)
    }

    #[payable]
    pub fn commit_owner(&mut self) {
        assert_one_yocto();

        if let Some(temp_owner) = self.temp_owner.clone() {
            require!(
                env::predecessor_account_id() == temp_owner,
                ERROR_UNAUTHORIZED
            )
        } else {
            panic!("{}", ERROR_TEMP_OWNER_NOT_SET);
        }
    }

    #[payable]
    pub fn update_operations_control(
        &mut self,
        update_operations_control_request: OperationsControlUpdateRequest,
    ) {
        assert_one_yocto();
        self.assert_owner_calling();

        self.operations_control.stake_paused = update_operations_control_request
            .stake_paused
            .unwrap_or(self.operations_control.stake_paused);
        self.operations_control.unstaked_paused = update_operations_control_request
            .unstake_paused
            .unwrap_or(self.operations_control.unstaked_paused);
        self.operations_control.withdraw_paused = update_operations_control_request
            .withdraw_paused
            .unwrap_or(self.operations_control.withdraw_paused);
        self.operations_control.epoch_stake_paused = update_operations_control_request
            .epoch_stake_paused
            .unwrap_or(self.operations_control.epoch_stake_paused);
        self.operations_control.epoch_unstake_paused = update_operations_control_request
            .epoch_unstake_paused
            .unwrap_or(self.operations_control.epoch_unstake_paused);
        self.operations_control.epoch_withdraw_paused = update_operations_control_request
            .epoch_withdraw_paused
            .unwrap_or(self.operations_control.epoch_withdraw_paused);
        self.operations_control.epoch_autocompounding_paused = update_operations_control_request
            .epoch_autocompounding_paused
            .unwrap_or(self.operations_control.epoch_autocompounding_paused);
    }

    // View methods

    pub fn get_account_staked_balance(&self, account_id: AccountId) -> U128 {
        self.get_account(account_id).staked_balance
    }

    pub fn get_account_total_balance(&self, account_id: AccountId) -> U128 {
        let acc = self.internal_get_account(&account_id);
        self.staked_amount_from_num_shares_rounded_down(acc.stake_shares)
            .into()
    }

    pub fn get_owner_id(&self) -> AccountId {
        self.owner_account_id.clone()
    }

    pub fn get_reward_fee_fraction(&self) -> Fraction {
        self.rewards_fee
    }

    pub fn set_reward_fee(&mut self, numerator: u32, denominator: u32) {
        self.assert_owner_calling();
        require!((numerator * 100 / denominator) < 20); // less than 20%
        self.rewards_fee = Fraction::new(numerator, denominator);
    }

    pub fn get_total_staked(&self) -> U128 {
        U128::from(self.total_staked)
    }

    pub fn get_staking_key(&self) -> PublicKey {
        panic!("{}", ERROR_NO_STAKING_KEY);
    }

    pub fn get_operations_control(&self) -> OperationControls {
        self.operations_control
    }

    pub fn get_account(&self, account_id: AccountId) -> AccountResponse {
        let account = self.internal_get_account(&account_id);
        AccountResponse {
            account_id,
            unstaked_balance: U128(account.unstaked_amount),
            staked_balance: self
                .staked_amount_from_num_shares_rounded_down(account.stake_shares)
                .into(),
            withdrawable_epoch: U64(account.withdrawable_epoch_height),
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
            total_staked: U128::from(self.total_staked),
            total_stake_shares: U128::from(self.total_stake_shares),
            accumulated_staked_rewards: U128::from(self.accumulated_staked_rewards),
            min_deposit_amount: U128::from(self.min_deposit_amount),
            operator_account_id: self.operator_account_id.clone(),
            rewards_fee_pct: self.rewards_fee,
            user_amount_to_stake_in_epoch: U128(self.user_amount_to_stake_in_epoch),
            user_amount_to_unstake_in_epoch: U128(self.user_amount_to_unstake_in_epoch),
            reconciled_epoch_stake_amount: U128(self.reconciled_epoch_stake_amount),
            reconciled_epoch_unstake_amount: U128(self.reconciled_epoch_unstake_amount),
            last_reconcilation_epoch: U64(self.last_reconcilation_epoch),
        }
    }

    pub fn get_nearx_price(&self) -> U128 {
        if self.total_staked == 0 || self.total_stake_shares == 0 {
            return U128(ONE_NEAR);
        }

        let amount = self.staked_amount_from_num_shares_rounded_down(ONE_NEAR);
        if amount == 0 {
            U128(ONE_NEAR)
        } else {
            U128(amount)
        }
    }

    pub fn get_validator_info(&self, validator: AccountId) -> ValidatorInfoResponse {
        let validator_info = if let Some(val_info) = self.validator_info_map.get(&validator) {
            val_info
        } else {
            ValidatorInfo::new(validator)
        };

        ValidatorInfoResponse {
            account_id: validator_info.account_id.clone(),
            staked: validator_info.staked.into(),
            unstaked: U128(validator_info.unstaked_amount),
            last_asked_rewards_epoch_height: validator_info.last_redeemed_rewards_epoch.into(),
            last_unstake_start_epoch: U64(validator_info.unstake_start_epoch),
            paused: validator_info.paused,
        }
    }

    pub fn get_validators(&self) -> Vec<ValidatorInfoResponse> {
        self.validator_info_map
            .iter()
            .map(|pool| ValidatorInfoResponse {
                account_id: pool.1.account_id.clone(),
                staked: U128::from(pool.1.staked),
                last_asked_rewards_epoch_height: U64(pool.1.last_redeemed_rewards_epoch),
                last_unstake_start_epoch: U64(pool.1.unstake_start_epoch),
                paused: pool.1.paused,
                unstaked: U128(pool.1.unstaked_amount),
            })
            .collect()
    }

    pub fn is_validator_unstake_pending(&self, validator: AccountId) -> bool {
        let validator_info = self.internal_get_validator(&validator);

        validator_info.pending_unstake_release()
    }

    pub fn get_current_epoch(&self) -> U64 {
        U64(env::epoch_height())
    }
}
