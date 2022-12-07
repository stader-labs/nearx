use crate::constants::NEW_VALIDATOR_MAP;
use crate::contract::*;
use near_sdk::*;
use std::str::FromStr;

#[near_bindgen]
impl NearxPool {
    /// Should only be called by this contract on migration.
    /// This is NOOP implementation. KEEP IT if you haven't changed contract state.
    /// If you have changed state, you need to implement migration from old state (keep the old
    /// struct with different name to deserialize it first).
    /// After migration goes live, revert back to this implementation for next updates.
    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        require!(env::state_exists());
        let old_contract = env::state_read::<LegacyNearxPoolV3>().expect("ERR_NOT_INITIALIZED");

        let mut new_validator_info_map = UnorderedMap::new(NEW_VALIDATOR_MAP.as_bytes());

        for old_validator in old_contract.validator_info_map.values() {
            let account_id = old_validator.account_id.clone();
            let new_validator_info =
                ValidatorInfoWrapper::LegacyValidatorInfo(old_validator).into_current();
            new_validator_info_map.insert(
                &account_id,
                &ValidatorInfoWrapper::ValidatorInfo(new_validator_info),
            );
        }

        let new_contract = NearxPool {
            owner_account_id: old_contract.owner_account_id,
            total_staked: old_contract.total_staked,
            total_stake_shares: old_contract.total_stake_shares,
            accumulated_staked_rewards: old_contract.accumulated_staked_rewards,
            user_amount_to_stake_in_epoch: old_contract.user_amount_to_stake_in_epoch,
            user_amount_to_unstake_in_epoch: old_contract.user_amount_to_unstake_in_epoch,
            reconciled_epoch_stake_amount: old_contract.reconciled_epoch_stake_amount,
            reconciled_epoch_unstake_amount: old_contract.reconciled_epoch_unstake_amount,
            last_reconcilation_epoch: old_contract.last_reconcilation_epoch,
            accounts: old_contract.accounts,
            validator_info_map: new_validator_info_map,
            total_validator_weight: old_contract.total_validator_weight,
            min_deposit_amount: old_contract.min_deposit_amount,
            operator_account_id: old_contract.operator_account_id,
            treasury_account_id: old_contract.treasury_account_id,
            rewards_fee: old_contract.rewards_fee,
            rewards_buffer: old_contract.rewards_buffer,
            accumulated_rewards_buffer: old_contract.accumulated_rewards_buffer,
            temp_owner: old_contract.temp_owner,
            temp_operator: old_contract.temp_operator,
            temp_treasury: old_contract.temp_treasury,
            temp_reward_fee: old_contract.temp_reward_fee,
            last_reward_fee_set_epoch: old_contract.last_reward_fee_set_epoch,
            operations_control: OperationControls {
                stake_paused: old_contract.operations_control.stake_paused,
                direct_stake_paused: false,
                unstaked_paused: old_contract.operations_control.unstaked_paused,
                withdraw_paused: old_contract.operations_control.withdraw_paused,
                staking_epoch_paused: old_contract.operations_control.staking_epoch_paused,
                unstaking_epoch_paused: old_contract.operations_control.unstaking_epoch_paused,
                withdraw_epoch_paused: old_contract.operations_control.withdraw_epoch_paused,
                autocompounding_epoch_paused: old_contract
                    .operations_control
                    .autocompounding_epoch_paused,
                sync_validator_balance_paused: old_contract
                    .operations_control
                    .sync_validator_balance_paused,
                ft_transfer_paused: old_contract.operations_control.ft_transfer_paused,
                ft_transfer_call_paused: old_contract.operations_control.ft_transfer_call_paused,
            },
            min_storage_reserve: old_contract.min_storage_reserve,
        };

        new_contract
    }
}

#[cfg(target_arch = "wasm32")]
mod upgrade {
    use near_sdk::Gas;
    use near_sys as sys;

    use super::*;

    /// Gas for completing the upgrade call
    pub const GAS_FOR_COMPLETING_UPGRADE_CALL: Gas = Gas(10 * 1_000_000_000_000);
    /// Minimum gas for calling state migration call. Please notice the gas cost will be higher
    /// if the number of accounts and validator pools grows.
    pub const MIN_GAS_FOR_MIGRATE_CALL: Gas = Gas(10 * 1_000_000_000_000);
    /// Gas for calling `get_summary` method
    pub const GAS_FOR_GET_SUMMARY_CALL: Gas = Gas(15 * 1_000_000_000_000);

    /// Self upgrade and call migrate, optimizes gas by not loading into memory the code.
    /// Takes as input non serialized set of bytes of the code.
    #[no_mangle]
    pub fn upgrade() {
        env::setup_panic_hook();
        let contract: NearxPool = env::state_read().expect("ERR_CONTRACT_IS_NOT_INITIALIZED");
        contract.assert_owner_calling();
        let current_id = env::current_account_id().as_bytes().to_vec();
        let migrate_method_name = b"migrate".to_vec();
        let get_summary_method_name = b"get_contract_summary".to_vec();
        unsafe {
            // Load input (wasm code) into register 0.
            sys::input(0);
            // Create batch action promise for the current contract ID
            let promise_id =
                sys::promise_batch_create(current_id.len() as _, current_id.as_ptr() as _);
            // 1st batch action in the Tx: "deploy contract" (code is taken from register 0)
            sys::promise_batch_action_deploy_contract(promise_id, u64::MAX as _, 0);
            // 2nd batch action in the Tx: call `migrate()` in the contract with sufficient gas
            let required_gas =
                env::used_gas() + GAS_FOR_COMPLETING_UPGRADE_CALL + GAS_FOR_GET_SUMMARY_CALL;
            require!(
                env::prepaid_gas() >= required_gas + MIN_GAS_FOR_MIGRATE_CALL,
                "Not enough gas to complete contract state migration"
            );
            let migrate_attached_gas = env::prepaid_gas() - required_gas;
            sys::promise_batch_action_function_call(
                promise_id,
                migrate_method_name.len() as _,
                migrate_method_name.as_ptr() as _,
                0 as _,
                0 as _,
                0 as _,
                migrate_attached_gas.0,
            );
            // 3rd batch action in the Tx: call `get_contract_summary()` in the contract to validate
            // the contract state. If the validation failed, the entire `upgrade()` method
            // will be rolled back. The `get_contract_summary()` view call will access most of the
            // states in the contract, so should guarantee the contract is working as expected
            sys::promise_batch_action_function_call(
                promise_id,
                get_summary_method_name.len() as _,
                get_summary_method_name.as_ptr() as _,
                0 as _,
                0 as _,
                0 as _,
                GAS_FOR_GET_SUMMARY_CALL.0,
            );
            sys::promise_return(promise_id);
        }
    }
}
