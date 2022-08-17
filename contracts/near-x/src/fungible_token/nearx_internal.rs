use crate::contract::NearxPool;
use crate::errors::{ERROR_REQUIRE_AMOUNT_GT_0, ERROR_SENDER_RECEIVER_SAME};
use near_sdk::json_types::U128;
use near_sdk::{env, log, require, AccountId, Balance, PromiseResult};

impl NearxPool {
    pub fn internal_nearx_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
    ) {
        require!(sender_id != receiver_id, ERROR_SENDER_RECEIVER_SAME);
        require!(amount > 0, ERROR_REQUIRE_AMOUNT_GT_0);
        let mut sender_acc = self.internal_get_account(sender_id);
        let mut receiver_acc = self.internal_get_account(receiver_id);
        assert!(
            amount <= sender_acc.stake_shares,
            "{} does not have enough NearX balance {}",
            sender_id,
            sender_acc.stake_shares
        );

        sender_acc.stake_shares -= amount;
        self.internal_update_account(sender_id, &sender_acc);

        receiver_acc.stake_shares += amount;
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
                receiver_acc.stake_shares -= refund_amount;
                self.internal_update_account(&receiver_id, &receiver_acc);

                let mut sender_acc = self.internal_get_account(sender_id);
                sender_acc.stake_shares += refund_amount;
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
