use crate::constants::{gas, NO_DEPOSIT};
use crate::contract::*;
use crate::events::Event;
use near_contract_standards::fungible_token::{
    core::FungibleTokenCore, metadata::FungibleTokenMetadata,
};
use near_sdk::{
    assert_one_yocto,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LazyOption, LookupMap},
    env, ext_contract,
    json_types::U128,
    log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue, StorageUsage,
};

#[ext_contract(ext_ft_receiver)]
pub trait FungibleTokenReceiver {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

#[ext_contract(ext_self)]
trait FungibleTokenResolver {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    metadata: LazyOption<FungibleTokenMetadata>,

    pub accounts: LookupMap<AccountId, Balance>,
    pub total_supply: Balance,
    pub account_storage_usage: StorageUsage,
}

#[near_bindgen]
impl FungibleTokenCore for NearxPool {
    /// NEP-141 for NEARX
    #[payable]
    fn ft_transfer(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        #[allow(unused)] memo: Option<String>,
    ) {
        assert_one_yocto();
        Event::FtTransfer {
            receiver_id: receiver_id.clone(),
            sender_id: env::predecessor_account_id(),
            amount,
        }
        .emit();
        self.internal_nearx_transfer(&env::predecessor_account_id(), &receiver_id, amount.0);
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        #[allow(unused)] memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert_one_yocto();
        let min_gas = gas::FT_TRANSFER + gas::FT_TRANSFER_RESOLVE;
        assert!(
            env::prepaid_gas() > min_gas,
            "require at least {:?} gas",
            min_gas
        );

        Event::FtTransferCall {
            receiver_id: receiver_id.clone(),
            sender_id: env::predecessor_account_id(),
            msg: msg.clone(),
            amount,
        }
        .emit();

        let receiver_id: AccountId = receiver_id;
        self.internal_nearx_transfer(&env::predecessor_account_id(), &receiver_id, amount.0);

        ext_ft_receiver::ext(receiver_id.clone())
            .with_attached_deposit(NO_DEPOSIT)
            .with_static_gas(env::prepaid_gas() - gas::FT_TRANSFER - gas::FT_TRANSFER_RESOLVE)
            .ft_on_transfer(env::predecessor_account_id(), amount, msg)
            .then(
                ext_self::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::FT_TRANSFER_RESOLVE)
                    .ft_resolve_transfer(env::predecessor_account_id(), receiver_id, amount),
            )
            .into()
    }

    fn ft_total_supply(&self) -> U128 {
        self.total_stake_shares.into()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.internal_get_account(&account_id).stake_shares.into()
    }
}

#[near_bindgen]
impl FungibleTokenResolver for NearxPool {
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let (used_amount, burned_amount) =
            self.int_ft_resolve_transfer(&sender_id, receiver_id, amount);
        if burned_amount > 0 {
            log!("{} tokens burned", burned_amount);
        }
        used_amount.into()
    }
}
