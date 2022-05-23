use near_contract_standards::fungible_token::{
    core::FungibleTokenCore,
    metadata::{FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC},
    resolver::FungibleTokenResolver,
};

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap};
use near_sdk::ext_contract;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{
    assert_one_yocto, env, log, near_bindgen, AccountId, Balance, Gas, PanicOnDefault,
    PromiseOrValue, StorageUsage,
};

use crate::*;

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

const GAS_FOR_FT_TRANSFER_CALL: u64 = 30_000_000_000_000;
const GAS_FOR_RESOLVE_TRANSFER: u64 = 11_000_000_000_000;
const FIVE_TGAS: u64 = 5_000_000_000_000;
const ONE_TGAS: u64 = 1_000_000_000_000;

const NO_DEPOSIT: Balance = 0;

fn ft_metadata_default() -> FungibleTokenMetadata {
    FungibleTokenMetadata {
        spec: FT_METADATA_SPEC.to_string(),
        name: "Stader and Near".to_string(),
        symbol: "NEARX".to_string(),
        icon: None,
        reference: Some("https://nearX.app".into()),
        reference_hash: None,
        decimals: 24,
    }
}
fn ft_metadata_init_lazy_container() -> LazyOption<FungibleTokenMetadata> {
    let metadata: LazyOption<FungibleTokenMetadata>;
    metadata = LazyOption::new(b"ftmd".to_vec(), None);
    return metadata;
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
    //NEP-141 for default token NEARX, ft_transfer
    /// Transfer `amount` of tokens from the caller of the contract (`predecessor_id`) to a contract at `receiver_id`.
    /// Requirements:
    /// * receiver_id must be a contract and must respond to `ft_on_transfer(&mut self, sender_id: AccountId, amount: U128String, _msg: String ) -> PromiseOrValue<U128>`
    /// * if receiver_id is not a contract or `ft_on_transfer` fails, the transfer is rolled-back
    #[payable]
    fn ft_transfer(
        &mut self,
        receiver_id: ValidAccountId,
        amount: U128,
        #[allow(unused)] memo: Option<String>,
    ) {
        assert_one_yocto();
        self.internal_nearx_transfer(
            &env::predecessor_account_id(),
            &receiver_id.into(),
            amount.0,
        );
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        amount: U128,
        #[allow(unused)] memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert_one_yocto();
        assert!(
            env::prepaid_gas() > GAS_FOR_FT_TRANSFER_CALL + GAS_FOR_RESOLVE_TRANSFER + FIVE_TGAS,
            "gas required {}",
            GAS_FOR_FT_TRANSFER_CALL + GAS_FOR_RESOLVE_TRANSFER + FIVE_TGAS
        );

        let receiver_id: AccountId = receiver_id.into();
        self.internal_nearx_transfer(
            &env::predecessor_account_id(),
            &receiver_id.clone(),
            amount.0,
        );

        ext_ft_receiver::ft_on_transfer(
            env::predecessor_account_id(),
            amount,
            msg,
            //promise params:
            &receiver_id, //contract
            NO_DEPOSIT,   //attached native NEAR amount
            env::prepaid_gas()
                - Gas::from(GAS_FOR_FT_TRANSFER_CALL)
                - Gas::from(GAS_FOR_RESOLVE_TRANSFER)
                - Gas::from(ONE_TGAS), // set almost all remaining gas for ft_on_transfer
        )
        .then(ext_self::ft_resolve_transfer(
            env::predecessor_account_id(),
            receiver_id.into(),
            amount,
            //promise params:
            &env::current_account_id(), //contract
            NO_DEPOSIT,                 //attached native NEAR amount
            GAS_FOR_RESOLVE_TRANSFER,
        ))
        .into()
    }

    //NearX total supply
    fn ft_total_supply(&self) -> U128 {
        self.total_stake_shares.into()
    }

    fn ft_balance_of(&self, account_id: ValidAccountId) -> U128 {
        let acc = self.internal_get_account(&account_id.into());
        return acc.stake_shares.into();
    }
}

#[near_bindgen]
impl FungibleTokenResolver for NearxPool {
    /// Returns the amount of burned tokens in a corner case when the sender
    /// has deleted (unregistered) their account while the `ft_transfer_call` was still in flight.
    /// Returns (Used token amount, Burned token amount)
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: ValidAccountId,
        receiver_id: ValidAccountId,
        amount: U128,
    ) -> U128 {
        let sender_id: AccountId = sender_id.into();
        let (used_amount, burned_amount) =
            self.int_ft_resolve_transfer(&sender_id, receiver_id.into(), amount);
        if burned_amount > 0 {
            log!("{} tokens burned", burned_amount);
        }
        return used_amount.into();
    }
}

#[near_bindgen]
impl FungibleTokenMetadataProvider for NearxPool {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        let metadata = ft_metadata_init_lazy_container();
        //load from storage or return default
        return metadata.get().unwrap_or(ft_metadata_default());
    }
}

#[near_bindgen]
impl NearxPool {
    pub fn ft_metadata_set(&self, data: FungibleTokenMetadata) {
        let mut metadata = ft_metadata_init_lazy_container();
        metadata.set(&data); //save into storage
    }
}
