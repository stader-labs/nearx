mod context;
mod helpers;

use crate::helpers::{abs_diff_eq, ntoy};
use context::IntegrationTestContext;
use near_sdk::json_types::{U128, U64};
use near_units::*;
use near_x::constants::gas::ON_STAKE_POOL_WITHDRAW_ALL_CB;
use near_x::constants::{NUM_EPOCHS_TO_UNLOCK, ONE_NEAR};
use near_x::state::{AccountResponse, Fraction, NearxPoolStateResponse, ValidatorInfoResponse};
use serde_json::json;

#[tokio::test]
async fn test_validator_removal() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3).await?;

    // Add deposits to validator 1
    let current_epoch_1 = context.get_current_epoch().await?;

    // Fill up all validators
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user1_account.staked_balance, U128(ntoy(10)));
    assert_eq!(user2_account.staked_balance, U128(ntoy(10)));
    assert_eq!(user3_account.staked_balance, U128(ntoy(10)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    context.epoch_stake().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(30)));
    assert_eq!(validator2_info.staked, U128(ntoy(0)));
    assert_eq!(validator3_info.staked, U128(ntoy(0)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_staked_balance, U128(ntoy(30)));
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(0)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(0)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    // Pause validator 1
    context
        .pause_validator(&context.get_stake_pool_contract(0).id())
        .await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.paused, true);

    // drain unstake from validator 1
    context
        .drain_unstake(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(ntoy(30)));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_1);

    let stake_pool1_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    assert_eq!(stake_pool1_unstaked_balance, U128(ntoy(30)));

    context.worker.fast_forward(10000).await?;

    // drain withdraw from validator 1
    let contract_balance_before_drain_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context
        .drain_withdraw(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let contract_balance_after_drain_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;

    assert!(abs_diff_eq(
        contract_balance_after_drain_withdraw - contract_balance_before_drain_withdraw,
        ntoy(30),
        ntoy(1)
    ));

    let stake_pool1_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    assert_eq!(stake_pool1_unstaked_balance, U128(ntoy(0)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_1);

    // let nearx_state = context.get_nearx_state().await?;
    // assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));

    // remove validator 1 from set
    let all_validators = context.get_validators().await?;
    assert_eq!(all_validators.len(), 3);
    context
        .remove_validator(&context.get_stake_pool_contract(0).id())
        .await?;
    let all_validators = context.get_validators().await?;
    assert_eq!(all_validators.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_user_stake_autocompound_unstake_withdraw_flows_all_validators_involved(
) -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3).await?;

    let current_epoch_1 = context.get_current_epoch().await?;

    // Fill up all validators
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user1_account.staked_balance, U128(ntoy(10)));
    assert_eq!(user2_account.staked_balance, U128(ntoy(10)));
    assert_eq!(user3_account.staked_balance, U128(ntoy(10)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    context.epoch_stake().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(30)));
    assert_eq!(validator2_info.staked, U128(ntoy(0)));
    assert_eq!(validator3_info.staked, U128(ntoy(0)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_staked_balance, U128(ntoy(30)));
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(0)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(0)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    context.worker.fast_forward(1000).await?;

    let current_epoch_2 = context.get_current_epoch().await?;

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user1_account.staked_balance, U128(ntoy(20)));
    assert_eq!(user2_account.staked_balance, U128(ntoy(20)));
    assert_eq!(user3_account.staked_balance, U128(ntoy(20)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(60)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(60)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    context.epoch_stake().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(30)));
    assert_eq!(validator2_info.staked, U128(ntoy(30)));
    assert_eq!(validator3_info.staked, U128(ntoy(0)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_staked_balance, U128(ntoy(30)));
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(30)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(0)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(60)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(60)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    context.worker.fast_forward(1000).await?;

    let current_epoch_3 = context.get_current_epoch().await?;

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user1_account.staked_balance, U128(ntoy(30)));
    assert_eq!(user2_account.staked_balance, U128(ntoy(30)));
    assert_eq!(user3_account.staked_balance, U128(ntoy(30)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(90)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(90)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    context.epoch_stake().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(30)));
    assert_eq!(validator2_info.staked, U128(ntoy(30)));
    assert_eq!(validator3_info.staked, U128(ntoy(30)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_staked_balance, U128(ntoy(30)));
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(30)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(30)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(90)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(90)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_3);

    context.worker.fast_forward(1000).await?;

    let current_epoch_4 = context.get_current_epoch().await?;

    // Now we need to unstake enough to cross all validators
    context.unstake(&context.user1, U128(ntoy(25))).await?;
    context.unstake(&context.user2, U128(ntoy(25))).await?;
    context.unstake(&context.user3, U128(ntoy(25))).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user1_account.staked_balance, U128(ntoy(5)));
    assert_eq!(user2_account.staked_balance, U128(ntoy(5)));
    assert_eq!(user3_account.staked_balance, U128(ntoy(5)));
    assert_eq!(user1_account.unstaked_balance, U128(ntoy(25)));
    assert_eq!(user2_account.unstaked_balance, U128(ntoy(25)));
    assert_eq!(user3_account.unstaked_balance, U128(ntoy(25)));
    assert_eq!(user1_account.unstaked_balance, U128(ntoy(25)));
    assert_eq!(user2_account.unstaked_balance, U128(ntoy(25)));
    assert_eq!(user3_account.unstaked_balance, U128(ntoy(25)));
    assert_eq!(
        user1_account.withdrawable_epoch,
        U64(current_epoch_4.0 + NUM_EPOCHS_TO_UNLOCK)
    );
    assert_eq!(
        user2_account.withdrawable_epoch,
        U64(current_epoch_4.0 + NUM_EPOCHS_TO_UNLOCK)
    );
    assert_eq!(
        user3_account.withdrawable_epoch,
        U64(current_epoch_4.0 + NUM_EPOCHS_TO_UNLOCK)
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(15)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(15)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(75)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_3);

    context.epoch_unstake().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(15)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(15)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(45)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_4);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(ntoy(30)));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_4);
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    assert_eq!(validator2_info.staked, U128(ntoy(30)));
    assert_eq!(validator2_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator2_info.last_unstake_start_epoch, U64(0));
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator3_info.staked, U128(ntoy(30)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator3_info.last_unstake_start_epoch, U64(0));

    context.epoch_unstake().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(15)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(15)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(15)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_4);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(ntoy(30)));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_4);
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    assert_eq!(validator2_info.staked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(ntoy(30)));
    assert_eq!(validator2_info.last_unstake_start_epoch, current_epoch_4);
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator3_info.staked, U128(ntoy(30)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator3_info.last_unstake_start_epoch, U64(0));

    // TODO - bchain check for unstaked balance

    context.epoch_unstake().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(15)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(15)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_4);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(ntoy(30)));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_4);
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    assert_eq!(validator2_info.staked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(ntoy(30)));
    assert_eq!(validator2_info.last_unstake_start_epoch, current_epoch_4);
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator3_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(15)));
    assert_eq!(validator3_info.last_unstake_start_epoch, current_epoch_4);

    assert_eq!(
        context
            .is_validator_unstake_pending(context.get_stake_pool_contract(0).id().clone())
            .await?,
        true
    );
    assert_eq!(
        context
            .is_validator_unstake_pending(context.get_stake_pool_contract(1).id().clone())
            .await?,
        true
    );
    assert_eq!(
        context
            .is_validator_unstake_pending(context.get_stake_pool_contract(2).id().clone())
            .await?,
        true
    );

    // Now user 3 unstakes remaining amount. The unstake epoch wait time should be twice since all validators are in stake
    context.unstake(&context.user3, U128(ntoy(3))).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user3_account.staked_balance, U128(ntoy(2)));
    assert_eq!(user3_account.unstaked_balance, U128(ntoy(28)));
    assert_eq!(
        user3_account.withdrawable_epoch,
        U64(current_epoch_4.0 + 2 * NUM_EPOCHS_TO_UNLOCK + 1)
    );

    context.worker.fast_forward(10000).await?;

    let contract_before_withdraw_from_val1 = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context
        .epoch_withdraw(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let contract_after_withdraw_from_val1 = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    assert!(abs_diff_eq(
        contract_after_withdraw_from_val1 - contract_before_withdraw_from_val1,
        ntoy(30),
        ntoy(1)
    ));

    let contract_before_withdraw_from_val2 = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context
        .epoch_withdraw(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let contract_after_withdraw_from_val2 = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    assert!(abs_diff_eq(
        contract_after_withdraw_from_val2 - contract_before_withdraw_from_val2,
        ntoy(30),
        ntoy(1)
    ));

    let contract_before_withdraw_from_val3 = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context
        .epoch_withdraw(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let contract_after_withdraw_from_val3 = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    assert!(abs_diff_eq(
        contract_after_withdraw_from_val3 - contract_before_withdraw_from_val3,
        ntoy(15),
        ntoy(1)
    ));

    let user1_balance_before_withdraw = context.user1.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context.user1.view_account(&context.worker).await?.balance;
    assert!(abs_diff_eq(
        user1_balance_after_withdraw - user1_balance_before_withdraw,
        ntoy(25),
        ntoy(1)
    ));

    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?.balance;
    assert!(abs_diff_eq(
        user2_balance_after_withdraw - user2_balance_before_withdraw,
        ntoy(25),
        ntoy(1)
    ));

    Ok(())
}

#[tokio::test]
async fn test_user_stake_autocompound_unstake_withdraw_flows_across_epochs() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3).await?;

    // User 1 stakes
    context.deposit(&context.user1, ntoy(10)).await?;
    // User 2 stakes
    context.deposit(&context.user2, ntoy(10)).await?;
    // User 1 unstakes 5N
    context.unstake(&context.user1, U128(ntoy(5))).await?;
    // User 3 stakes
    context.deposit(&context.user3, ntoy(10)).await?;
    // User 2 unstakes 5N
    context.unstake(&context.user2, U128(ntoy(5))).await?;
    // User 3 unstakes 5N
    context.unstake(&context.user3, U128(ntoy(5))).await?;
    // User 1 stakes
    context.deposit(&context.user1, ntoy(10)).await?;
    // User 2 stakes
    context.deposit(&context.user2, ntoy(10)).await?;
    // User 3 stakes
    context.deposit(&context.user3, ntoy(10)).await?;
    // User 2 unstakes 4N
    context.unstake(&context.user3, U128(ntoy(4))).await?;

    let current_epoch_1 = context.get_current_epoch().await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(15)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(15)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(9)),
            staked_balance: U128(ntoy(11)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;
    assert_eq!(user1_token_balance, U128(ntoy(15)));
    assert_eq!(user2_token_balance, U128(ntoy(15)));
    assert_eq!(user3_token_balance, U128(ntoy(11)));

    let total_supply = context.get_total_tokens_supply().await?;
    assert_eq!(total_supply, U128(ntoy(41)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(41)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(41)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(60)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(19)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: validator1_info.account_id.clone(),
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: validator2_info.account_id.clone(),
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: validator3_info.account_id.clone(),
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );

    // epoch stake
    context.epoch_stake().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(41)));
    assert_eq!(validator2_info.staked, U128(ntoy(0)));
    assert_eq!(validator3_info.staked, U128(ntoy(0)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_staked_balance, U128(ntoy(41)));
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(0)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(0)));

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(1)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(41)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(41)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));

    // User 1 stakes
    context.deposit(&context.user1, ntoy(10)).await?;
    // User 2 stakes
    context.deposit(&context.user2, ntoy(10)).await?;
    // User 3 stakes
    context.deposit(&context.user3, ntoy(10)).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(25)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(25)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(9)),
            staked_balance: U128(ntoy(21)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    // epoch autocompound add 20N rewards
    context
        .add_stake_pool_rewards(U128(ntoy(20)), context.get_stake_pool_contract(0))
        .await?;

    context
        .auto_compound_rewards(context.get_stake_pool_contract(0).id())
        .await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(61)));
    assert_eq!(validator2_info.staked, U128(ntoy(0)));
    assert_eq!(validator3_info.staked, U128(ntoy(0)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_staked_balance, U128(ntoy(61)));
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(0)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(0)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(91)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(71)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(20)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(1281690140845070422535211));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(32042253521126760563380281),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(32042253521126760563380281),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(9)),
            staked_balance: U128(26915492957746478873239436),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    // User 3 unstakes 10N
    context.unstake(&context.user3, U128(ntoy(5))).await?;

    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(14000000000000000000000001),
            staked_balance: U128(21915492957746478873239436),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(85999999999999999999999999));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(67098901098901098901098901)
    );
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(20)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(
        nearx_state.user_amount_to_unstake_in_epoch,
        U128(5000000000000000000000001)
    );
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(1281690140845070422535211));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(61)));
    assert_eq!(validator2_info.staked, U128(ntoy(0)));
    assert_eq!(validator3_info.staked, U128(ntoy(0)));
    assert_eq!(validator1_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator2_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(0)));

    // epoch unstake
    context.epoch_unstake().await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(85999999999999999999999999));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(67098901098901098901098901)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(
        nearx_state.user_amount_to_unstake_in_epoch,
        U128(5000000000000000000000001)
    );
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let stake_pool_1_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_unstaked_balance, U128(0));
    assert_eq!(stake_pool_2_unstaked_balance, U128(0));
    assert_eq!(stake_pool_3_unstaked_balance, U128(0));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(
        stake_pool_1_staked_balance,
        U128(61000000000000000000000000)
    );
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(0)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(0)));

    // fast forward by 1000
    context.worker.fast_forward(1000).await?;
    let current_epoch_2 = context.get_current_epoch().await?;

    // epoch stake
    context.epoch_stake().await?;

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(
        stake_pool_1_staked_balance,
        U128(61000000000000000000000000)
    );
    assert_eq!(
        stake_pool_2_staked_balance,
        U128(24999999999999999999999999)
    );
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(0)));

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(85999999999999999999999999));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(67098901098901098901098901)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(1281690140845070422535211));

    // epoch autocompound add 10N rewards
    context
        .add_stake_pool_rewards(U128(ntoy(5)), context.get_stake_pool_contract(0))
        .await?;
    context
        .add_stake_pool_rewards(U128(ntoy(5)), context.get_stake_pool_contract(1))
        .await?;

    context
        .auto_compound_rewards(context.get_stake_pool_contract(0).id())
        .await?;
    context
        .auto_compound_rewards(context.get_stake_pool_contract(1).id())
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(95999999999999999999999999));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(67098901098901098901098901)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(30)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(
        stake_pool_1_staked_balance,
        U128(66000000000000000000000000)
    );
    assert_eq!(
        stake_pool_2_staked_balance,
        U128(29999999999999999999999999)
    );
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(0)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(66000000000000000000000000));
    assert_eq!(validator2_info.staked, U128(29999999999999999999999999));
    assert_eq!(validator3_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator2_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(0)));

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(1430723878152636750736980));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;
    println!("user1_token_balance is {:?}", user1_token_balance);
    println!("user2_token_balance is {:?}", user2_token_balance);
    println!("user3_token_balance is {:?}", user3_token_balance);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);
    println!("Checked user accounts");

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(35768096953815918768424500),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(35768096953815918768424500),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(14000000000000000000000001),
            staked_balance: U128(24463806092368162463150998),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );

    // epoch unstake
    let res = context.epoch_unstake().await?;
    assert_eq!(res.json::<bool>().unwrap(), false);

    // epoch withdraw
    context.worker.fast_forward(10000).await?;
    let current_epoch_3 = context.get_current_epoch().await?;
    println!("current_epoch_3 is {:?}", current_epoch_3);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    if validator1_info.unstaked.0 != 0 {
        context
            .epoch_withdraw(context.get_stake_pool_contract(0).id().clone())
            .await?;
    }
    if validator2_info.unstaked.0 != 0 {
        context
            .epoch_withdraw(context.get_stake_pool_contract(1).id().clone())
            .await?;
    }
    if validator3_info.unstaked.0 != 0 {
        context
            .epoch_withdraw(context.get_stake_pool_contract(2).id().clone())
            .await?;
    }

    // user 1 unstakes 1N
    context.unstake(&context.user1, U128(ntoy(1))).await?;

    // user 2 withdraws
    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?.balance;
    println!(
        "user2_balance_before_withdraw is {}",
        user2_balance_before_withdraw
    );
    println!(
        "user2_balance_after_withdraw is {}",
        user2_balance_after_withdraw
    );

    // user 3 withdraws
    let user3_balance_before_withdraw = context.user3.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user3).await?;
    let user3_balance_after_withdraw = context.user3.view_account(&context.worker).await?.balance;
    println!(
        "user3_balance_before_withdraw is {}",
        user3_balance_before_withdraw
    );
    println!(
        "user3_balance_after_withdraw is {}",
        user3_balance_after_withdraw
    );

    // User 3 unstakes 5N
    context.unstake(&context.user3, U128(ntoy(5))).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);
    println!("Checked user accounts");

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(6000000000000000000000001),
            staked_balance: U128(34768096953815918768424499),
            withdrawable_epoch: U64(current_epoch_3.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(35768096953815918768424499),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(5000000000000000000000001),
            staked_balance: U128(19463806092368162463150997),
            withdrawable_epoch: U64(current_epoch_3.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    Ok(())
}

/// Happy flows of testing
#[tokio::test]
async fn test_user_direct_stake_unstake_withdraw_flows_in_same_epoch() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3).await?;

    // User 1 deposits 10N
    context
        .deposit_direct_stake(&context.user1, ntoy(10))
        .await?;
    // User 2 deposits 10N
    context
        .deposit_direct_stake(&context.user2, ntoy(10))
        .await?;

    // User 1 unstakes 3 N
    context.unstake(&context.user1, U128(ntoy(3))).await?;
    // User 3 deposits 10N
    context
        .deposit_direct_stake(&context.user3, ntoy(10))
        .await?;

    // User 3 unstakes 5N
    context.unstake(&context.user3, U128(ntoy(5))).await?;
    // User 1 deposits 10N
    context
        .deposit_direct_stake(&context.user1, ntoy(10))
        .await?;
    // User 2 deposits 10N
    context
        .deposit_direct_stake(&context.user2, ntoy(10))
        .await?;
    // User 2 unstakes 10N
    context.unstake(&context.user2, U128(ntoy(10))).await?;
    // User 1 unstaked 10N
    context.unstake(&context.user1, U128(ntoy(10))).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    let nearx_state = context.get_nearx_state().await?;
    println!(
        "reconcilation epoch is {:?}",
        nearx_state.last_reconcilation_epoch
    );
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));
    assert_eq!(nearx_state.total_staked, U128(ntoy(22)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(22)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(28)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));

    let current_epoch = context.get_current_epoch().await?;
    println!("current epoch is {:?}", current_epoch);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(13)),
            staked_balance: U128(ntoy(7)),
            withdrawable_epoch: user1_account.withdrawable_epoch
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: context.user2.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(10)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: user2_account.withdrawable_epoch
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: context.user3.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: user3_account.withdrawable_epoch
        }
    );

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(7)));
    assert_eq!(user2_token_balance, U128(ntoy(10)));
    assert_eq!(user3_token_balance, U128(ntoy(5)));

    let total_token_supply = context.get_total_tokens_supply().await?;
    assert_eq!(total_token_supply, U128(ntoy(22)));

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(20)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(20)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(10)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(0)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(20)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(1)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(20)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(2)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(10)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );

    // now we run unstake epoch
    let res = context.epoch_unstake().await?;
    println!("res for first call is {:?}", res);
    let res = context.epoch_unstake().await?;
    println!("res for second call is {:?}", res);

    let last_unstake_epoch = context.get_current_epoch().await?;

    let stake_pool_contract_unstaked_balance_0 = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_unstaked_balance_1 = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_unstaked_balance_2 = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_contract_unstaked_balance_0, U128(ntoy(20)));
    assert_eq!(stake_pool_contract_unstaked_balance_1, U128(ntoy(8)));
    assert_eq!(stake_pool_contract_unstaked_balance_2, U128(ntoy(0)));

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    println!("Stake pool balances after unstake");
    println!(
        "stake_pool_contract_balance_0 is {:?}",
        stake_pool_contract_balance_0
    );
    println!(
        "stake_pool_contract_balance_1 is {:?}",
        stake_pool_contract_balance_1
    );
    println!(
        "stake_pool_contract_balance_2 is {:?}",
        stake_pool_contract_balance_2
    );

    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(0)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(12)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(10)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("Validator accounts after unstake");
    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(0)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(0)),
            unstaked: U128(ntoy(20)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: current_epoch,
            paused: false
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(1)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(12)),
            unstaked: U128(ntoy(8)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: current_epoch,
            paused: false
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(2)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(10)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );

    context.worker.fast_forward(5000).await?;

    println!(
        "epoch_withdraw for val 0 {:?}",
        context.get_stake_pool_contract(0).id()
    );
    context
        .epoch_withdraw(context.get_stake_pool_contract(0).id().clone())
        .await?;
    println!("epoch_withdraw for val 1");
    context
        .epoch_withdraw(context.get_stake_pool_contract(1).id().clone())
        .await?;

    let user1_call = context.withdraw_all(&context.user1).await?;
    let user2_call = context.withdraw_all(&context.user2).await?;
    let user3_call = context.withdraw_all(&context.user3).await?;
    assert!(user1_call.is_success());
    assert!(user2_call.is_success());
    assert!(user3_call.is_success());

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    println!("User accounts after withdrawal");
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(7)),
            withdrawable_epoch: user1_account.withdrawable_epoch
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: context.user2.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: user2_account.withdrawable_epoch
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: context.user3.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: user3_account.withdrawable_epoch
        }
    );

    context.worker.fast_forward(1000).await?;

    // Now user does batched staking
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(0),
            staked_balance: U128(ntoy(17)),
            withdrawable_epoch: user1_account.withdrawable_epoch
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(0),
            staked_balance: U128(ntoy(20)),
            withdrawable_epoch: user2_account.withdrawable_epoch
        }
    );

    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(0),
            staked_balance: U128(ntoy(15)),
            withdrawable_epoch: user3_account.withdrawable_epoch
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    println!(
        "reconcilation epoch is {:?}",
        nearx_state.last_reconcilation_epoch
    );
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch);
    assert_eq!(nearx_state.total_staked, U128(ntoy(52)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(52)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));

    context.epoch_stake().await?;

    let nearx_state = context.get_nearx_state().await?;
    println!(
        "reconcilation epoch is {:?}",
        nearx_state.last_reconcilation_epoch
    );
    let current_epoch = context.get_current_epoch().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch);
    assert_eq!(nearx_state.total_staked, U128(ntoy(52)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(52)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    println!("Stake pool balances after unstake");
    println!(
        "stake_pool_contract_balance_0 is {:?}",
        stake_pool_contract_balance_0
    );
    println!(
        "stake_pool_contract_balance_1 is {:?}",
        stake_pool_contract_balance_1
    );
    println!(
        "stake_pool_contract_balance_2 is {:?}",
        stake_pool_contract_balance_2
    );

    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(30)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(12)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(10)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("Validator accounts after unstake");
    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(0)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(30)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: last_unstake_epoch,
            paused: false
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(1)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(12)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: last_unstake_epoch,
            paused: false
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(2)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(10)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );

    Ok(())
}

#[tokio::test]
async fn test_user_stake_unstake_withdraw_flows_in_same_epoch() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3).await?;

    // User 1 deposit
    // User 2 deposit
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;

    // User 1 unstakes 3 N
    context.unstake(&context.user1, U128(ntoy(3))).await?;
    // User 3 deposits
    context.deposit(&context.user3, ntoy(10)).await?;

    // User 3 unstakes 5N
    context.unstake(&context.user3, U128(ntoy(5))).await?;
    // User 1 deposits again
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.unstake(&context.user2, U128(ntoy(10))).await?;

    let current_epoch = context.get_current_epoch().await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(3)),
            staked_balance: U128(ntoy(17)),
            withdrawable_epoch: U64(current_epoch.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: context.user2.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(10)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(current_epoch.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: context.user3.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(current_epoch.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(17)));
    assert_eq!(user2_token_balance, U128(ntoy(10)));
    assert_eq!(user3_token_balance, U128(ntoy(5)));

    let total_token_supply = context.get_total_tokens_supply().await?;
    assert_eq!(total_token_supply, U128(ntoy(32)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));
    assert_eq!(nearx_state.total_staked, U128(ntoy(32)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(32)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(50)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(18)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(0)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(0)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(0)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(0)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(0)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(1)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(0)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(2)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(0)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );

    let current_epoch = context.get_current_epoch().await?;

    // Run stake epoch
    context.epoch_stake().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch);
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(0)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(32)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(1)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(0)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(2)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(0)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(32)));
    assert_eq!(stake_pool_contract_balance_1, U128(0));
    assert_eq!(stake_pool_contract_balance_2, U128(0));

    let stake_pool_contract_unstaked_balance_0 = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_unstaked_balance_1 = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_unstaked_balance_2 = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_contract_unstaked_balance_0, U128(0));
    assert_eq!(stake_pool_contract_unstaked_balance_1, U128(0));
    assert_eq!(stake_pool_contract_unstaked_balance_2, U128(0));

    // now we run unstake epoch
    context.epoch_unstake().await?;

    let stake_pool_contract_unstaked_balance_0 = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_unstaked_balance_1 = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_unstaked_balance_2 = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_contract_unstaked_balance_0, U128(0));
    assert_eq!(stake_pool_contract_unstaked_balance_1, U128(0));
    assert_eq!(stake_pool_contract_unstaked_balance_2, U128(0));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(0)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(32)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(1)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(0)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(2)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(ntoy(0)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );

    // User withdraws before withdrawable time
    // let user1_call = context.withdraw_all(&context.user1).await?;
    // let user2_call = context.withdraw_all(&context.user2).await?;
    // let user3_call = context.withdraw_all(&context.user3).await?;
    // assert!(user1_call.is_failure());
    // assert!(user2_call.is_failure());
    // assert!(user3_call.is_failure());

    context.worker.fast_forward(5000).await?;

    let user1_call = context.withdraw_all(&context.user1).await?;
    let user2_call = context.withdraw_all(&context.user2).await?;
    let user3_call = context.withdraw_all(&context.user3).await?;
    assert!(user1_call.is_success());
    assert!(user2_call.is_success());
    assert!(user3_call.is_success());

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(17)),
            withdrawable_epoch: U64(current_epoch.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: context.user2.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(current_epoch.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: context.user3.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(current_epoch.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    Ok(())
}

// Tests: Deposit and stake with epoch
#[tokio::test]
async fn test_deposit_and_stake_with_epoch() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3).await?;

    // Deposit for the 3 users
    // Add user deposits
    println!("User 1 depositing");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    context.deposit(&context.user2, ntoy(10)).await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    context.deposit(&context.user3, ntoy(10)).await?;
    println!("User 3 successfully deposited");

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(10)));
    assert_eq!(user2_staked_amount, U128(ntoy(10)));
    assert_eq!(user3_staked_amount, U128(ntoy(10)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(10)));
    assert_eq!(user2_token_balance, U128(ntoy(10)));
    assert_eq!(user3_token_balance, U128(ntoy(10)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(0)));
    assert_eq!(validator2_info.staked, U128(ntoy(0)));
    assert_eq!(validator3_info.staked, U128(ntoy(0)));

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_contract_balance_0, U128(0));
    assert_eq!(stake_pool_contract_balance_1, U128(0));
    assert_eq!(stake_pool_contract_balance_2, U128(0));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));

    // Run epoch stake
    context.epoch_stake().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(30)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(
        nearx_state.last_reconcilation_epoch,
        context.get_current_epoch().await?
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(30)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(0)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(0)));

    Ok(())
}

// Tests: Unstake with a withdraw following up
#[tokio::test]
async fn test_stake_unstake_and_withdraw_flow_happy_flow() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3).await?;

    // Deposit for the 3 users
    // Add user deposits
    println!("User 1 depositing");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    context.deposit(&context.user2, ntoy(10)).await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    context.deposit(&context.user3, ntoy(10)).await?;
    println!("User 3 successfully deposited");

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(10)));
    assert_eq!(user2_staked_amount, U128(ntoy(10)));
    assert_eq!(user3_staked_amount, U128(ntoy(10)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(10)));
    assert_eq!(user2_token_balance, U128(ntoy(10)));
    assert_eq!(user3_token_balance, U128(ntoy(10)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(0)));
    assert_eq!(validator2_info.staked, U128(ntoy(0)));
    assert_eq!(validator3_info.staked, U128(ntoy(0)));

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_contract_balance_0, U128(0));
    assert_eq!(stake_pool_contract_balance_1, U128(0));
    assert_eq!(stake_pool_contract_balance_2, U128(0));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));

    // Run epoch stake
    context.epoch_stake().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(30)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(
        nearx_state.last_reconcilation_epoch,
        context.get_current_epoch().await?
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(30)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(0)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(0)));

    // User 1 and User 2 unstake 5 NEAR each
    context.unstake(&context.user1, U128(ntoy(5))).await?;
    context.unstake(&context.user2, U128(ntoy(5))).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    let current_epoch = context.get_current_epoch().await?;
    println!("current_epoch is {:?}", current_epoch);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(current_epoch.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: context.user2.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(current_epoch.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: context.user3.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(5)));
    assert_eq!(user2_token_balance, U128(ntoy(5)));
    assert_eq!(user3_token_balance, U128(ntoy(10)));

    let total_token_supply = context.get_total_tokens_supply().await?;
    assert_eq!(total_token_supply, U128(ntoy(20)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(10)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(20)));
    assert_eq!(nearx_state.total_staked, U128(ntoy(20)));

    context.worker.fast_forward(5000).await?;

    let validator_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    println!(
        "validator unstaked balance before unstake epoch is {:?}",
        validator_unstaked_balance
    );
    assert_eq!(validator_unstaked_balance, U128(ntoy(0)));

    // Run the unstake epoch
    context.epoch_unstake().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(20)));
    assert_eq!(nearx_state.total_staked, U128(ntoy(20)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));

    let current_epoch = context.get_current_epoch().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(20)));
    assert_eq!(validator1_info.unstaked, U128(ntoy(10)));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch);

    let validator_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    println!(
        "validator unstaked balance after unstake epoch is {:?}",
        validator_unstaked_balance
    );
    assert_eq!(validator_unstaked_balance, U128(ntoy(10)));

    // Run the withdraw epoch after 4 epochs to get the amount back
    // Check the contract balance before and after the withdraw call

    context.worker.fast_forward(5000).await?;

    let initial_contract_balance = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    println!("initial contract balance is {:?}", initial_contract_balance);
    let res = context
        .epoch_withdraw(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let balance_after_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    println!("balance after withdraw is {:?}", balance_after_withdraw);
    // assert!(is_close(
    //     balance_after_withdraw - initial_contract_balance,
    //     10 * ONE_NEAR
    // ));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.unstaked, U128(ntoy(0)));

    let validator_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    println!(
        "validator unstaked balance after unstake epoch is {:?}",
        validator_unstaked_balance
    );
    assert_eq!(validator_unstaked_balance, U128(ntoy(0)));

    println!("total gas burnt is {:?}", res.total_gas_burnt);

    // User withdraw flow
    let user1_balance = context.user1.view_account(&context.worker).await?.balance;
    println!("initial user balance is {:?}", user1_balance);
    let res = context.withdraw_all(&context.user1).await?;
    println!("res is {:?}", res);
    println!("withdrawal gas burnt {:?}", res.outcome());
    let user1_balance_after_withdrawal = context.user1.view_account(&context.worker).await?.balance;
    println!(
        "user balance after withdrawal is {:?}",
        user1_balance_after_withdrawal
    );
    println!(
        "diff in user balance after withdrawal is {:?}",
        ((user1_balance_after_withdrawal) - user1_balance)
    );
    // assert!(is_close(
    //     user1_balance_after_withdrawal - user1_balance,
    //     10 * ONE_NEAR
    // ));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    assert_eq!(user1_account.unstaked_balance, U128(ntoy(0)));

    // User withdraw flow
    let user2_balance = context.user2.view_account(&context.worker).await?.balance;
    println!("initial user balance is {:?}", user2_balance);
    let res = context.withdraw_all(&context.user2).await?;
    println!("res is {:?}", res);
    println!("withdrawal gas burnt {:?}", res.outcome());
    let user2_balance_after_withdrawal = context.user2.view_account(&context.worker).await?.balance;
    println!(
        "user balance after withdrawal is {:?}",
        user2_balance_after_withdrawal
    );
    println!(
        "diff in user balance after withdrawal is {:?}",
        ((user2_balance_after_withdrawal) - user2_balance)
    );
    // assert!(is_close(
    //     user2_balance_after_withdrawal - user2_balance,
    //     10 * ONE_NEAR
    // ));

    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    assert_eq!(user2_account.unstaked_balance, U128(ntoy(0)));

    Ok(())
}

// Tests: Autocompound with operator rewards and autocompound in the same epoch
#[tokio::test]
async fn test_autocompound_with_operator_rewards() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3).await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator2_info.staked, U128(0));
    assert_eq!(validator3_info.staked, U128(0));

    // Add user deposits
    println!("User 1 depositing");
    context
        .deposit_direct_stake(&context.user1, ntoy(10))
        .await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    context
        .deposit_direct_stake(&context.user2, ntoy(10))
        .await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    context
        .deposit_direct_stake(&context.user3, ntoy(10))
        .await?;
    println!("User 3 successfully deposited");

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(10)));
    assert_eq!(user2_staked_amount, U128(ntoy(10)));
    assert_eq!(user3_staked_amount, U128(ntoy(10)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(10)));
    assert_eq!(user2_token_balance, U128(ntoy(10)));
    assert_eq!(user3_token_balance, U128(ntoy(10)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(10)));
    assert_eq!(validator2_info.staked, U128(ntoy(10)));
    assert_eq!(validator3_info.staked, U128(ntoy(10)));

    let total_token_supply = context.get_total_tokens_supply().await?;
    assert_eq!(total_token_supply, U128(ntoy(30)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(0)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));

    // Set reward fee to 10%
    context
        .set_reward_fee(Fraction {
            numerator: 10,
            denominator: 100,
        })
        .await?;
    let reward_fee = context.get_reward_fee().await?;
    assert_eq!(reward_fee.numerator, 10);
    assert_eq!(reward_fee.denominator, 100);

    // Add 30Near of rewards
    context
        .add_stake_pool_rewards(U128(ntoy(30)), context.get_stake_pool_contract(0))
        .await?;

    // Get the operator details
    let operator_account = context
        .worker
        .view_account(&context.nearx_operator.id())
        .await?;
    let previous_operator_balance = operator_account.balance;

    // auto compound the rewards
    context
        .auto_compound_rewards(context.get_stake_pool_contract(0).id())
        .await?;

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(2)));

    let validator = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    println!("validator is {:?}", validator);
    assert_eq!(
        validator,
        ValidatorInfoResponse {
            account_id: validator.account_id.clone(),
            staked: U128(ntoy(40)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: context.get_current_epoch().await?,
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(60)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));

    let operator_account = context
        .worker
        .view_account(&context.nearx_operator.id())
        .await?;
    let current_operator_balance = operator_account.balance;

    assert_eq!(
        (current_operator_balance - previous_operator_balance),
        ntoy(3)
    );

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(10)));
    assert_eq!(user2_token_balance, U128(ntoy(10)));
    assert_eq!(user3_token_balance, U128(ntoy(10)));

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(20)));
    assert_eq!(user2_staked_amount, U128(ntoy(20)));
    assert_eq!(user3_staked_amount, U128(ntoy(20)));

    // Deposit with NearX price > 1
    println!("User 1 depositing");
    context
        .deposit_direct_stake(&context.user1, ntoy(10))
        .await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    context
        .deposit_direct_stake(&context.user2, ntoy(10))
        .await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    context
        .deposit_direct_stake(&context.user3, ntoy(10))
        .await?;
    println!("User 3 successfully deposited");

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(30)));
    assert_eq!(user2_staked_amount, U128(ntoy(30)));
    assert_eq!(user3_staked_amount, U128(ntoy(30)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(15)));
    assert_eq!(user2_token_balance, U128(ntoy(15)));
    assert_eq!(user3_token_balance, U128(ntoy(15)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(40)));
    assert_eq!(validator2_info.staked, U128(ntoy(30)));
    assert_eq!(validator3_info.staked, U128(ntoy(20)));

    let total_token_supply = context.get_total_tokens_supply().await?;
    assert_eq!(total_token_supply, U128(ntoy(45)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(90)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));

    // Autocompound in the same epoch
    let operator_account = context
        .worker
        .view_account(&context.nearx_operator.id())
        .await?;
    let previous_operator_balance = operator_account.balance;

    context
        .auto_compound_rewards(context.get_stake_pool_contract(0).id())
        .await?;

    let operator_account = context
        .worker
        .view_account(&context.nearx_operator.id())
        .await?;
    let current_operator_balance = operator_account.balance;

    let validator = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    println!("validator is {:?}", validator);
    assert_eq!(
        validator,
        ValidatorInfoResponse {
            account_id: validator.account_id.clone(),
            staked: U128(ntoy(40)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: context.get_current_epoch().await?,
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(90)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(2)));

    assert_eq!((current_operator_balance - previous_operator_balance), 0);

    Ok(())
}

// Tests: Autocompound with no stake
#[tokio::test]
async fn test_autocompound_with_no_stake() -> anyhow::Result<()> {
    println!("***** Step 1: Initialization *****");
    let context = IntegrationTestContext::new(3).await?;

    // Auto compound
    println!("autocompounding!");
    context
        .auto_compound_rewards(context.get_stake_pool_contract(0).id())
        .await?;
    println!("done autocompounding!");

    println!("getting nearx_price");
    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(1)));
    println!("getting nearx_state");
    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(0));
    assert_eq!(nearx_state.total_staked, U128(0));
    assert_eq!(nearx_state.total_stake_shares, U128(0));

    println!("getting validator info");
    let validator_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator_info.staked, U128(0));
    assert_eq!(validator_info.last_asked_rewards_epoch_height, U64(0));

    Ok(())
}

#[tokio::test]
async fn test_deposit_flows() -> anyhow::Result<()> {
    println!("***** Step 1: Initialization *****");
    let context = IntegrationTestContext::new(3).await?;

    // First test
    // user1, user2 and user3 deposit 10 NEAR each. We check whether the staking contract
    // Check initial deposits
    println!("**** Step 2: User deposit test ****");
    println!("Checking initial user deposits");

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;
    assert_eq!(user1_staked_amount, U128(0));
    assert_eq!(user2_staked_amount, U128(0));
    assert_eq!(user3_staked_amount, U128(0));

    println!("Successfully checked initial user deposits");

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator2_info.staked, U128(0));
    assert_eq!(validator3_info.staked, U128(0));

    println!("**** Simulating user deposits ****");
    println!("User 1 depositing");
    context
        .deposit_direct_stake(&context.user1, ntoy(10))
        .await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    context
        .deposit_direct_stake(&context.user2, ntoy(10))
        .await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    context
        .deposit_direct_stake(&context.user3, ntoy(10))
        .await?;
    println!("User 3 successfully deposited");

    println!("Checking user deposits after users have deposited");
    let user1_staked_amount = context.get_user_account(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_account(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount.staked_balance, U128(ntoy(10)));
    assert_eq!(user2_staked_amount.staked_balance, U128(ntoy(10)));
    assert_eq!(user3_staked_amount.staked_balance, U128(ntoy(10)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(10)));
    assert_eq!(user2_token_balance, U128(ntoy(10)));
    assert_eq!(user3_token_balance, U128(ntoy(10)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(10)));
    assert_eq!(validator2_info.staked, U128(ntoy(10)));
    assert_eq!(validator3_info.staked, U128(ntoy(10)));

    let nearx_price = context.get_nearx_price().await?;
    println!("nearx_price is {:?}", nearx_price);
    assert_eq!(nearx_price, U128(ntoy(1)));

    let total_staked_amount = context.get_total_staked_amount().await?;
    println!("total_staked_amount is {:?}", total_staked_amount);
    assert_eq!(total_staked_amount, U128(ntoy(30)));

    let total_tokens_minted = context.get_total_tokens_supply().await?;
    assert_eq!(total_tokens_minted, U128(ntoy(30)));

    // Second test
    // Test token transfers
    println!("**** Step 3: Token transferring ****");

    println!("Successfully checked initial user deposits");

    println!("User 1 transfers 5 tokens to User 2");
    context
        .ft_transfer(&context.user1, &context.user2, ntoy(5).to_string())
        .await?;
    println!("User 2 transfers 3 tokens to User 3");
    context
        .ft_transfer(&context.user2, &context.user3, ntoy(3).to_string())
        .await?;
    println!("User 3 transfers 1 token to User 1");
    context
        .ft_transfer(&context.user3, &context.user1, ntoy(1).to_string())
        .await?;

    println!("Checking user deposits after users have deposited");
    let user1_staked_amount = context.get_user_account(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_account(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount.staked_balance, U128(ntoy(6)));
    assert_eq!(user2_staked_amount.staked_balance, U128(ntoy(12)));
    assert_eq!(user3_staked_amount.staked_balance, U128(ntoy(12)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(6)));
    assert_eq!(user2_token_balance, U128(ntoy(12)));
    assert_eq!(user3_token_balance, U128(ntoy(12)));

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(1)));

    let total_staked_amount = context.get_total_staked_amount().await?;
    assert_eq!(total_staked_amount, U128(ntoy(30)));

    let total_tokens_minted = context.get_total_tokens_supply().await?;
    assert_eq!(total_tokens_minted, U128(ntoy(30)));

    println!("**** Step 4: Auto compounding ****");

    println!("Fast forward 61400 blocks");
    // context.worker.fast_forward(61400).await?;

    println!("Auto compounding stake pool");

    // Adding rewards
    context
        .add_stake_pool_rewards(U128(ntoy(30)), context.get_stake_pool_contract(0))
        .await?;

    // restake_staking_pool(&worker, &stake_pool_contract).await?;
    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state before auto compounding is {:?}", nearx_state);

    println!("Auto compounding nearx pool");
    context
        .auto_compound_rewards(context.get_stake_pool_contract(0).id())
        .await?;
    //
    let nearx_price = context.get_nearx_price().await?;
    // println!("nearx_price is {:?}", nearx_price);
    assert_eq!(nearx_price, U128(ntoy(2)));

    let total_tokens_minted = context.get_total_tokens_supply().await?;
    assert_eq!(total_tokens_minted, U128(ntoy(30)));

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(12)));
    assert_eq!(user2_staked_amount, U128(ntoy(24)));
    assert_eq!(user3_staked_amount, U128(ntoy(24)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(6)));
    assert_eq!(user2_token_balance, U128(ntoy(12)));
    assert_eq!(user3_token_balance, U128(ntoy(12)));

    let validator = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    println!("validator is {:?}", validator);
    assert_eq!(
        validator,
        ValidatorInfoResponse {
            account_id: validator.account_id.clone(),
            staked: U128(ntoy(40)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: context.get_current_epoch().await?,
            last_unstake_start_epoch: U64(0),
            paused: false
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(60)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));

    println!("nearx_state after auto compounding is {:?}", nearx_state);

    Ok(())
}
