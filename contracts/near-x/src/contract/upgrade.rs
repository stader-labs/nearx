use crate::contract::*;
use crate::*;
use near_sdk::*;

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
        let contract: NearxPool = env::state_read().expect("ERR_NOT_INITIALIZED");
        contract
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
