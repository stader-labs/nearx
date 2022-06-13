use crate::errors::*;
use crate::{
    constants::{gas, MIN_BALANCE_FOR_STORAGE, MIN_UNSTAKE_AMOUNT, NO_DEPOSIT},
    contract::*,
    errors,
    state::*,
};
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::{log, require, AccountId, Balance, Promise, PromiseOrValue};

const EMPTY_STORAGE_BALANCE: StorageBalance = StorageBalance {
    total: U128 { 0: 0 },
    available: U128 { 0: 0 },
};

#[near_bindgen]
impl StorageManagement for NearxPool {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        if env::attached_deposit() > 0 {
            Promise::new(env::predecessor_account_id()).transfer(env::attached_deposit())
        }
        EMPTY_STORAGE_BALANCE
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        if env::attached_deposit() > 0 {
            Promise::new(env::predecessor_account_id()).transfer(env::attached_deposit())
        }
        EMPTY_STORAGE_BALANCE
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        if env::attached_deposit() > 0 {
            Promise::new(env::predecessor_account_id()).transfer(env::attached_deposit())
        }
        true
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        todo!()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        todo!()
    }
}
