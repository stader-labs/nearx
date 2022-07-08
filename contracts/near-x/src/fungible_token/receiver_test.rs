use near_sdk::json_types::{U128, U64};
use near_sdk::{
    env, near_bindgen,
    serde::{Deserialize, Serialize},
    serde_json, AccountId, PromiseOrValue,
};

use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::collections::ERR_ELEMENT_DESERIALIZATION;

use crate::contract::*;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TestReciever {
    pub amount: U128,
}

#[near_bindgen]
impl FungibleTokenReceiver for NearxPool {
    /// Callback on receiving tokens by this contract.
    /// transfer reward token with specific msg indicate
    /// which farm to be deposited to.
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let message =
            serde_json::from_str::<TestReciever>(&msg).expect(ERR_ELEMENT_DESERIALIZATION);
        PromiseOrValue::Value(message.amount)
    }
}
