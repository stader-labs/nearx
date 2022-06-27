use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds};
use near_sdk::json_types::U128;
use near_sdk::{env, near_bindgen, require, AccountId, Promise};

use crate::contract::*;

const EMPTY_STORAGE_BALANCE: StorageBalance = StorageBalance {
    total: U128(0),
    available: U128(0),
};

#[near_bindgen]
impl NearxPool {
    #[allow(unused_variables)]
    #[payable]
    pub fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        if env::attached_deposit() > 0 {
            Promise::new(env::predecessor_account_id()).transfer(env::attached_deposit());
        }
        EMPTY_STORAGE_BALANCE
    }

    /// * returns a `storage_balance` struct if `amount` is 0
    pub fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        if let Some(amount) = amount {
            if amount.0 > 0 {
                require!(
                    true,
                    "The amount is greater than the available storage balance"
                );
            }
        }
        StorageBalance {
            total: 0.into(),
            available: 0.into(),
        }
    }

    #[allow(unused_variables)]
    pub fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        true
    }

    pub fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: U128(0),
            max: Some(U128(0)),
        }
    }

    #[allow(unused_variables)]
    pub fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        Some(EMPTY_STORAGE_BALANCE)
    }
}
