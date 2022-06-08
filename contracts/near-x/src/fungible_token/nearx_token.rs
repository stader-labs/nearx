use crate::contract::*;
use near_contract_standards::fungible_token::{
    core::FungibleTokenCore,
    metadata::{FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC},
};
use near_sdk::{
    assert_one_yocto,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LazyOption, LookupMap},
    env, ext_contract,
    json_types::U128,
    log, near_bindgen, AccountId, Balance, Gas, PanicOnDefault, PromiseOrValue, PromiseResult,
    StorageUsage,
};
use crate::constants::NO_DEPOSIT;

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

const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas(30_000_000_000_000);
const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(11_000_000_000_000);
const FIVE_TGAS: Gas = Gas(5_000_000_000_000);
const ONE_TGAS: Gas = Gas(1_000_000_000_000);

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
        assert!(
            env::prepaid_gas() > GAS_FOR_FT_TRANSFER_CALL + GAS_FOR_RESOLVE_TRANSFER + FIVE_TGAS,
            "require at least {:?} gas",
            GAS_FOR_FT_TRANSFER_CALL + GAS_FOR_RESOLVE_TRANSFER + FIVE_TGAS
        );

        let receiver_id: AccountId = receiver_id;
        self.internal_nearx_transfer(&env::predecessor_account_id(), &receiver_id, amount.0);

        ext_ft_receiver::ext(receiver_id.clone())
            .with_attached_deposit(NO_DEPOSIT)
            .with_static_gas(
                env::prepaid_gas() - GAS_FOR_FT_TRANSFER_CALL - GAS_FOR_RESOLVE_TRANSFER - ONE_TGAS,
            )
            .ft_on_transfer(env::predecessor_account_id(), amount, msg)
            .then(
                ext_self::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
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

#[near_bindgen]
impl NearxPool {
    pub fn internal_nearx_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
    ) {
        assert!(amount > 0, "The amount should be a positive number");
        let mut sender_acc = self.internal_get_account(sender_id);
        let mut receiver_acc = self.internal_get_account(receiver_id);
        assert!(
            amount <= sender_acc.stake_shares,
            "{} does not have enough NearX balance {}",
            sender_id,
            sender_acc.stake_shares
        );

        sender_acc.sub_stake_shares(amount);
        receiver_acc.add_stake_shares(amount);

        self.internal_update_account(sender_id, &sender_acc);
        self.internal_update_account(receiver_id, &receiver_acc);
    }

    pub fn int_ft_resolve_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> (u128, u128) {
        let receiver_id = receiver_id;
        let amount: Balance = amount.into();

        // Get the unused amount from the `ft_on_transfer` call result.
        let unused_amount = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount, unused_amount.0)
                } else {
                    amount
                }
            }
            PromiseResult::Failed => amount,
        };

        if unused_amount > 0 {
            let mut receiver_acc = self.internal_get_account(&receiver_id);
            let receiver_balance = receiver_acc.stake_shares;
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                receiver_acc.sub_stake_shares(refund_amount);
                self.internal_update_account(&receiver_id, &receiver_acc);

                let mut sender_acc = self.internal_get_account(sender_id);
                sender_acc.add_stake_shares(refund_amount);
                self.internal_update_account(sender_id, &sender_acc);

                log!(
                    "Refund {} from {} to {}",
                    refund_amount,
                    receiver_id,
                    sender_id
                );
                return (amount - refund_amount, 0);
            }
        }
        (amount, 0)
    }
}
