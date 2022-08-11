use crate::contract::*;
use crate::state::Account;
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::{assert_one_yocto, env, log, AccountId, Balance, Promise};

/// Fixed amount of storage bytes. The contract is light weight in terms of storage amount.
/// The below storage spec should cover all storage needs of the contract
const STORAGE_AMOUNT_BYTES: usize = 250;

impl NearxPool {
    pub(crate) fn internal_storage_balance_of(
        &self,
        account_id: &AccountId,
    ) -> Option<StorageBalance> {
        if self.accounts.get(account_id).is_some() {
            Some(StorageBalance {
                total: self.storage_balance_bounds().min,
                available: 0.into(),
            })
        } else {
            None
        }
    }

    pub(crate) fn internal_register_account(&mut self, account_id: &AccountId) {
        if self
            .accounts
            .insert(account_id, &Account::default())
            .is_some()
        {
            env::panic_str("The account is already registered");
        }
    }
}

#[allow(unused_variables)]
#[near_bindgen]
impl StorageManagement for NearxPool {
    // `registration_only` doesn't affect the implementation for vanilla fungible token.
    #[allow(unused_variables)]
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        let amount: Balance = env::attached_deposit();
        let account_id = account_id.unwrap_or_else(env::predecessor_account_id);
        if let Some(account) = self.accounts.get(&account_id) {
            log!("The account is already registered, refunding the deposit");
            if amount > 0 {
                Promise::new(env::predecessor_account_id()).transfer(amount);
            }
        } else {
            let min_balance = self.storage_balance_bounds().min.0;
            if amount < min_balance {
                log!("min_balance is {}", min_balance);
                log!("amount is {}", amount);
                env::panic_str("The attached deposit is less than the minimum storage balance");
            }

            self.internal_register_account(&account_id);
            let refund = amount - min_balance;
            if refund > 0 {
                Promise::new(env::predecessor_account_id()).transfer(refund);
            }
        }
        self.internal_storage_balance_of(&account_id).unwrap()
    }

    /// While storage_withdraw normally allows the caller to retrieve `available` balance, the basic
    /// Fungible Token implementation sets storage_balance_bounds.min == storage_balance_bounds.max,
    /// which means available balance will always be 0. So this implementation:
    /// * panics if `amount > 0`
    /// * never transfers Ⓝ to caller
    /// * returns a `storage_balance` struct if `amount` is 0
    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        assert_one_yocto();
        let predecessor_account_id = env::predecessor_account_id();
        if let Some(storage_balance) = self.internal_storage_balance_of(&predecessor_account_id) {
            match amount {
                Some(amount) if amount.0 > 0 => {
                    env::panic_str("The amount is greater than the available storage balance");
                }
                _ => storage_balance,
            }
        } else {
            env::panic_str(
                format!("The account {} is not registered", &predecessor_account_id).as_str(),
            );
        }
    }

    /// Storage unregister is disabled because staking users don't need
    /// to deposit but they are allowed to withdraw storage fee with
    /// the current implementation.
    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        panic!("Storage unregister is not supported yet.");
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        let required_storage_balance = STORAGE_AMOUNT_BYTES as Balance * env::storage_byte_cost();
        StorageBalanceBounds {
            min: required_storage_balance.into(),
            max: Some(required_storage_balance.into()),
        }
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.internal_storage_balance_of(&account_id)
    }
}
