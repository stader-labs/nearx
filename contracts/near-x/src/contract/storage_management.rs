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

#[near_bindgen]
impl StorageManagement for NearxPool {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        todo!()
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        todo!()
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        todo!()
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        todo!()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        todo!()
    }
}
