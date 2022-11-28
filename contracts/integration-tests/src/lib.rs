mod constants;
mod context;
mod helpers;
mod legacy_types;

use crate::constants::ONE_EPOCH;
use crate::helpers::{abs_diff_eq, ntoy};
use context::IntegrationTestContext;
use near_sdk::json_types::{U128, U64};
use near_sdk::{AccountId, ONE_NEAR};
use near_units::*;
use near_x::constants::gas::ON_STAKE_POOL_WITHDRAW_ALL_CB;
use near_x::constants::NUM_EPOCHS_TO_UNLOCK;
use near_x::state::{
    AccountResponse, Fraction, HumanReadableAccount, NearxPoolStateResponse,
    OperationsControlUpdateRequest, ValidatorInfoResponse, ValidatorType,
};
use serde_json::json;
use std::str::FromStr;
use workspaces::network::DevAccountDeployer;

// Important data points to check
/// 1. nearx state
/// 2. nearx price
/// 3. user account
/// 4. validator account
/// 5. actual staked info
/// 6. actual unstaked info
///

#[tokio::test]
async fn test_set_owner() -> anyhow::Result<()> {
    let mut context = IntegrationTestContext::new(3, None).await?;

    let new_owner_account = context.worker.dev_create_account().await?;
    let new_owner_near_sdk_account =
        near_sdk::AccountId::new_unchecked(new_owner_account.id().to_string());

    context.set_owner(&new_owner_account.id().clone()).await?;

    let roles = context.get_roles().await?;
    assert_eq!(roles.temp_owner, Some(new_owner_near_sdk_account.clone()));

    context.commit_owner(&new_owner_account).await?;

    let roles = context.get_roles().await?;

    assert_eq!(roles.owner_account, new_owner_near_sdk_account);

    let roles = context.get_roles().await?;
    assert!(roles.temp_owner.is_none());

    Ok(())
}

#[tokio::test]
async fn test_set_operator() -> anyhow::Result<()> {
    let mut context = IntegrationTestContext::new(3, None).await?;

    let new_operator_account = context.worker.dev_create_account().await?;
    let new_operator_near_sdk_account =
        near_sdk::AccountId::new_unchecked(new_operator_account.id().to_string());

    context
        .set_operator(&new_operator_account.id().clone())
        .await?;

    let roles = context.get_roles().await?;
    assert_eq!(
        roles.temp_operator,
        Some(new_operator_near_sdk_account.clone())
    );

    context.commit_operator(&new_operator_account).await?;

    let roles = context.get_roles().await?;

    assert_eq!(roles.operator_account, new_operator_near_sdk_account);

    let roles = context.get_roles().await?;
    assert!(roles.temp_operator.is_none());

    Ok(())
}

#[tokio::test]
async fn test_set_treasury() -> anyhow::Result<()> {
    let mut context = IntegrationTestContext::new(3, None).await?;

    let new_treasury_account = context.worker.dev_create_account().await?;
    let new_treasury_near_sdk_account =
        near_sdk::AccountId::new_unchecked(new_treasury_account.id().to_string());

    println!("setting treasury");
    context
        .set_treasury(&new_treasury_account.id().clone())
        .await?;

    let roles = context.get_roles().await?;
    assert_eq!(
        roles.temp_treasury,
        Some(new_treasury_near_sdk_account.clone())
    );

    println!("commit treasury!");
    context.commit_treasury(&new_treasury_account).await?;

    let roles = context.get_roles().await?;

    assert_eq!(roles.treasury_account, new_treasury_near_sdk_account);

    let roles = context.get_roles().await?;
    assert!(roles.temp_treasury.is_none());

    Ok(())
}

#[tokio::test]
async fn test_reward_fee_set() -> anyhow::Result<()> {
    let mut context = IntegrationTestContext::new(3, None).await?;

    context.set_reward_fee(Fraction::new(6, 100)).await?;

    let nearx_pool_state = context.get_nearx_state().await?;
    assert_eq!(nearx_pool_state.temp_reward_fee.unwrap().numerator, 6);
    assert_eq!(nearx_pool_state.temp_reward_fee.unwrap().denominator, 100);

    context.worker.fast_forward(ONE_EPOCH).await?;

    assert!(context.commit_reward_fee().await.is_err());

    context.worker.fast_forward(5 * ONE_EPOCH).await?;

    context.commit_reward_fee().await?;

    let nearx_pool_state = context.get_nearx_state().await?;
    assert_eq!(nearx_pool_state.rewards_fee_pct.numerator, 6);
    assert_eq!(nearx_pool_state.rewards_fee_pct.denominator, 100);
    assert!(nearx_pool_state.temp_reward_fee.is_none());

    Ok(())
}

#[ignore]
#[tokio::test]
async fn test_contract_upgrade() -> anyhow::Result<()> {
    let old_contract = "./../../res/near_x_50273033d58cf3b61532b9703d7b7110a1e09071.wasm";

    println!("Deploying old contract!");
    let mut context = IntegrationTestContext::new(3, Some(old_contract)).await?;

    let current_epoch_1 = context.get_current_epoch().await?;

    let new_contract = "./../../res/near_x.wasm";

    println!("depositing!");
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let current_epoch = context.get_current_epoch().await?;
    println!("validator1 is {:?}", validator1_info);
    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: validator1_info.account_id.clone(),
            staked: U128(ntoy(15)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: validator2_info.account_id.clone(),
            staked: U128(ntoy(15)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: validator3_info.account_id.clone(),
            staked: U128(ntoy(15)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    context.worker.fast_forward(3 * ONE_EPOCH).await?;

    let current_epoch_2 = context.get_current_epoch().await?;

    context.unstake(&context.user1, U128(ntoy(5))).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(40)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(40)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(5)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(40)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(40)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let current_epoch = context.get_current_epoch().await?;
    println!("validator1 is {:?}", validator1_info);
    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: validator1_info.account_id.clone(),
            staked: U128(ntoy(10)),
            unstaked: U128(ntoy(5)),
            last_asked_rewards_epoch_height: current_epoch_2,
            last_unstake_start_epoch: U64(current_epoch_2.0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(10)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: validator2_info.account_id.clone(),
            staked: U128(ntoy(15)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch_2,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: validator3_info.account_id.clone(),
            staked: U128(ntoy(15)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch_2,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    context.unstake(&context.user2, U128(ntoy(5))).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    println!("user2_account is {:?}", user2_account);
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );

    println!("Reading the new contract!");
    let nearx_2_wasm = std::fs::read(new_contract)?;

    context.upgrade(nearx_2_wasm).await?;

    let user2_account = context.get_account(context.user2.id().clone()).await?;
    assert_eq!(user2_account.unstaked_balance, U128(ntoy(5)));
    assert_eq!(user2_account.staked_balance, U128(ntoy(5)));
    assert_eq!(user2_account.can_withdraw, false);

    // test set_min_storage_reserve
    context.add_min_storage_reserve(U128(ntoy(60))).await?;

    context.worker.fast_forward(2 * ONE_EPOCH).await?;

    let current_epoch_3 = context.get_current_epoch().await?;

    // test reward buffer update
    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);
    assert_eq!(nearx_state.total_staked, U128(ntoy(35)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(35)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(5)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(0));
    assert_eq!(nearx_state.accumulated_rewards_buffer, U128(0));
    assert_eq!(nearx_state.min_storage_reserve, U128(ntoy(60)));

    context.update_rewards_buffer(ntoy(5)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);
    assert_eq!(nearx_state.total_staked, U128(ntoy(40)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(35)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(5)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(ntoy(5)));
    assert_eq!(nearx_state.accumulated_rewards_buffer, U128(ntoy(5)));

    let nearx_price = context.get_nearx_price().await?;
    println!("nearx_price is {:?}", nearx_price);
    assert_eq!(nearx_price, U128(1142857142857142857142857));

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_3);
    assert_eq!(nearx_state.total_staked, U128(ntoy(40)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(35)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(0));
    assert_eq!(nearx_state.accumulated_rewards_buffer, U128(ntoy(5)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let current_epoch_4 = context.get_current_epoch().await?;
    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: validator1_info.account_id.clone(),
            staked: U128(ntoy(10)),
            unstaked: U128(ntoy(5)),
            last_asked_rewards_epoch_height: current_epoch_4,
            last_unstake_start_epoch: U64(current_epoch_2.0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(10)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: validator2_info.account_id.clone(),
            staked: U128(ntoy(15)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: current_epoch_4,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: validator3_info.account_id.clone(),
            staked: U128(ntoy(15)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch_4,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    Ok(())
}

#[tokio::test]
async fn test_all_epochs_paused() -> anyhow::Result<()> {
    let mut context = IntegrationTestContext::new(3, None).await?;

    let current_epoch = context.get_current_epoch().await?;

    context
        .update_operation_controls(OperationsControlUpdateRequest {
            stake_paused: None,
            direct_stake_paused: None,
            unstake_paused: None,
            withdraw_paused: None,
            staking_epoch_paused: Some(true),
            unstaking_epoch_paused: Some(true),
            withdraw_epoch_paused: Some(true),
            autocompounding_epoch_paused: Some(true),
            sync_validator_balance_paused: Some(true),
            ft_transfer_paused: Some(true),
            ft_transfer_call_paused: Some(true),
        })
        .await?;

    let operations_controls = context.get_operations_controls().await?;
    assert_eq!(operations_controls.autocompounding_epoch_paused, true);
    assert_eq!(operations_controls.staking_epoch_paused, true);
    assert_eq!(operations_controls.unstaking_epoch_paused, true);
    assert_eq!(operations_controls.withdraw_epoch_paused, true);
    assert_eq!(operations_controls.ft_transfer_paused, true);
    assert_eq!(operations_controls.ft_transfer_call_paused, true);

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    context.unstake(&context.user1, U128(ntoy(5))).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(40)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(40)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(5)));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(user1_account.staked_balance, U128(ntoy(5)));
    assert_eq!(user1_account.unstaked_balance, U128(ntoy(5)));
    assert_eq!(
        user1_account.withdrawable_epoch,
        U64(current_epoch.0 + NUM_EPOCHS_TO_UNLOCK)
    );
    assert_eq!(user2_account.staked_balance, U128(ntoy(10)));
    assert_eq!(user3_account.staked_balance, U128(ntoy(10)));

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(40)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(40)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(5)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    context.worker.fast_forward(6 * ONE_EPOCH).await?;

    let user1_balance_before_withdraw = context.user1.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context.user1.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        (user1_balance_after_withdraw - user1_balance_before_withdraw),
        ntoy(5),
        100000000000000000000000
    ));

    Ok(())
}

#[tokio::test]
async fn test_system_with_no_validators() -> anyhow::Result<()> {
    let mut context = IntegrationTestContext::new(0, None).await?;

    // deposit and unstake
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    context.unstake(&context.user1, U128(ntoy(5))).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(25)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(25)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(5)));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(user1_account.staked_balance, U128(ntoy(5)));
    assert_eq!(user1_account.unstaked_balance, U128(ntoy(5)));
    assert_eq!(user2_account.staked_balance, U128(ntoy(10)));
    assert_eq!(user3_account.staked_balance, U128(ntoy(10)));

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(25)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(25)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(25)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));

    context.worker.fast_forward(ONE_EPOCH).await?;

    context.unstake(&context.user1, U128(ntoy(5))).await?;
    context.unstake(&context.user2, U128(ntoy(5))).await?;
    context.unstake(&context.user3, U128(ntoy(5))).await?;
    context.unstake(&context.user3, U128(ntoy(5))).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(user1_account.staked_balance, U128(0));
    assert_eq!(user1_account.unstaked_balance, U128(ntoy(10)));
    assert_eq!(user2_account.staked_balance, U128(ntoy(5)));
    assert_eq!(user2_account.unstaked_balance, U128(ntoy(5)));
    assert_eq!(user3_account.staked_balance, U128(0));
    assert_eq!(user3_account.unstaked_balance, U128(ntoy(10)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(5)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(5)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(20)));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(user1_account.staked_balance, U128(ntoy(0)));
    assert_eq!(user1_account.unstaked_balance, U128(ntoy(10)));
    assert_eq!(user2_account.staked_balance, U128(ntoy(5)));
    assert_eq!(user2_account.unstaked_balance, U128(ntoy(5)));
    assert_eq!(user3_account.staked_balance, U128(ntoy(0)));
    assert_eq!(user3_account.unstaked_balance, U128(ntoy(10)));

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(5)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(5)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(5)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));

    context.worker.fast_forward(5 * ONE_EPOCH).await?;

    context.run_epoch_methods().await?;

    let user1_balance_before_withdraw = context.user1.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context.user1.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        (user1_balance_after_withdraw - user1_balance_before_withdraw),
        ntoy(10),
        ntoy(1)
    ));

    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        (user2_balance_after_withdraw - user2_balance_before_withdraw),
        ntoy(5),
        ntoy(1)
    ));

    let user3_balance_before_withdraw = context.user3.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user3).await?;
    let user3_balance_after_withdraw = context.user3.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        (user3_balance_after_withdraw - user3_balance_before_withdraw),
        ntoy(10),
        ntoy(1)
    ));

    Ok(())
}

#[tokio::test]
async fn test_direct_deposit_and_stake() -> anyhow::Result<()> {
    let mut context = IntegrationTestContext::new(3, None).await?;

    context
        .make_validator_private(context.get_stake_pool_contract(0).id(), None)
        .await?;
    context
        .make_validator_private(context.get_stake_pool_contract(1).id(), None)
        .await?;
    context
        .make_validator_private(context.get_stake_pool_contract(2).id(), None)
        .await?;

    context
        .direct_deposit_and_stake(
            &context.user1,
            ntoy(10),
            context.get_stake_pool_contract(0).id(),
        )
        .await?;
    context
        .direct_deposit_and_stake(
            &context.user2,
            ntoy(10),
            context.get_stake_pool_contract(1).id(),
        )
        .await?;
    context
        .direct_deposit_and_stake(
            &context.user3,
            ntoy(10),
            context.get_stake_pool_contract(2).id(),
        )
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    println!("got stake pool staked amount");

    assert_eq!(stake_pool_1_amount, U128(ntoy(15)));
    assert_eq!(stake_pool_2_amount, U128(ntoy(15)));
    assert_eq!(stake_pool_3_amount, U128(ntoy(15)));

    Ok(())
}

#[tokio::test]
async fn test_validator_selection_with_0_weight() -> anyhow::Result<()> {
    let mut context = IntegrationTestContext::new(3, None).await?;

    context.add_validator(20).await?;
    context.add_validator(20).await?;

    let total_validator_weight = context.get_total_validator_weight().await?;
    assert_eq!(total_validator_weight, 70);

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(65)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(65)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(50)));

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let validator4_info = context
        .get_validator_info(context.get_stake_pool_contract(3).id().clone())
        .await?;
    let validator5_info = context
        .get_validator_info(context.get_stake_pool_contract(4).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);
    println!("validator4_info is {:?}", validator4_info);
    println!("validator5_info is {:?}", validator5_info);

    assert_eq!(validator1_info.staked, U128(9285714285714285714285714));
    assert_eq!(validator2_info.staked, U128(9285714285714285714285714));
    assert_eq!(validator3_info.staked, U128(9285714285714285714285716));
    assert_eq!(validator4_info.staked, U128(18571428571428571428571428));
    assert_eq!(validator5_info.staked, U128(18571428571428571428571428));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator4_info.unstaked, U128(0));
    assert_eq!(validator5_info.unstaked, U128(0));

    context.worker.fast_forward(ONE_EPOCH).await?;

    context
        .pause_validator(context.get_stake_pool_contract(0).id())
        .await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.weight, 0);

    let total_validator_weight = context.get_total_validator_weight().await?;
    assert_eq!(total_validator_weight, 60);

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(115)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(115)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(50)));

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let validator4_info = context
        .get_validator_info(context.get_stake_pool_contract(3).id().clone())
        .await?;
    let validator5_info = context
        .get_validator_info(context.get_stake_pool_contract(4).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);
    println!("validator4_info is {:?}", validator4_info);
    println!("validator5_info is {:?}", validator5_info);

    assert_eq!(validator1_info.staked, U128(9285714285714285714285714));
    assert_eq!(validator2_info.staked, U128(19761904761904761904761904));
    assert_eq!(validator3_info.staked, U128(9285714285714285714285716));
    assert_eq!(validator4_info.staked, U128(38333333333333333333333333));
    assert_eq!(validator5_info.staked, U128(38333333333333333333333333));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator4_info.unstaked, U128(0));
    assert_eq!(validator5_info.unstaked, U128(0));

    context.worker.fast_forward(ONE_EPOCH).await?;

    context
        .pause_validator(context.get_stake_pool_contract(1).id())
        .await?;

    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    assert_eq!(validator2_info.weight, 0);

    let total_validator_weight = context.get_total_validator_weight().await?;
    assert_eq!(total_validator_weight, 50);

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(165)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(165)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(50)));

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let validator4_info = context
        .get_validator_info(context.get_stake_pool_contract(3).id().clone())
        .await?;
    let validator5_info = context
        .get_validator_info(context.get_stake_pool_contract(4).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);
    println!("validator4_info is {:?}", validator4_info);
    println!("validator5_info is {:?}", validator5_info);

    assert_eq!(validator1_info.staked, U128(9285714285714285714285714));
    assert_eq!(validator2_info.staked, U128(19761904761904761904761904));
    assert_eq!(validator3_info.staked, U128(31619047619047619047619049));
    assert_eq!(validator4_info.staked, U128(66000000000000000000000000));
    assert_eq!(validator5_info.staked, U128(38333333333333333333333333));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator4_info.unstaked, U128(0));
    assert_eq!(validator5_info.unstaked, U128(0));

    context.worker.fast_forward(ONE_EPOCH).await?;

    context
        .pause_validator(context.get_stake_pool_contract(2).id())
        .await?;

    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator3_info.weight, 0);

    let total_validator_weight = context.get_total_validator_weight().await?;
    assert_eq!(total_validator_weight, 40);

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(215)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(215)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(50)));

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let validator4_info = context
        .get_validator_info(context.get_stake_pool_contract(3).id().clone())
        .await?;
    let validator5_info = context
        .get_validator_info(context.get_stake_pool_contract(4).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);
    println!("validator4_info is {:?}", validator4_info);
    println!("validator5_info is {:?}", validator5_info);

    assert_eq!(validator1_info.staked, U128(9285714285714285714285714));
    assert_eq!(validator2_info.staked, U128(19761904761904761904761904));
    assert_eq!(validator3_info.staked, U128(31619047619047619047619049));
    assert_eq!(validator4_info.staked, U128(66000000000000000000000000));
    assert_eq!(validator5_info.staked, U128(88333333333333333333333333));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator4_info.unstaked, U128(0));
    assert_eq!(validator5_info.unstaked, U128(0));

    /// All validators are paused
    context.worker.fast_forward(ONE_EPOCH).await?;

    context
        .pause_validator(context.get_stake_pool_contract(3).id())
        .await?;
    context
        .pause_validator(context.get_stake_pool_contract(4).id())
        .await?;

    context.deposit(&context.user1, ntoy(10)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(225)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(225)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(10)));

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(225)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(225)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(10)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let validator4_info = context
        .get_validator_info(context.get_stake_pool_contract(3).id().clone())
        .await?;
    let validator5_info = context
        .get_validator_info(context.get_stake_pool_contract(4).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);
    println!("validator4_info is {:?}", validator4_info);
    println!("validator5_info is {:?}", validator5_info);

    assert_eq!(validator1_info.staked, U128(9285714285714285714285714));
    assert_eq!(validator2_info.staked, U128(19761904761904761904761904));
    assert_eq!(validator3_info.staked, U128(31619047619047619047619049));
    assert_eq!(validator4_info.staked, U128(66000000000000000000000000));
    assert_eq!(validator5_info.staked, U128(88333333333333333333333333));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator4_info.unstaked, U128(0));
    assert_eq!(validator5_info.unstaked, U128(0));

    Ok(())
}

#[tokio::test]
async fn test_validator_selection() -> anyhow::Result<()> {
    let mut context = IntegrationTestContext::new(3, None).await?;

    /// All validators have equal weight
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    // Update validator weights to 1:2:3
    context
        .update_validator(context.get_stake_pool_contract(0).id().clone(), 10)
        .await?;
    context
        .update_validator(context.get_stake_pool_contract(1).id().clone(), 20)
        .await?;
    context
        .update_validator(context.get_stake_pool_contract(2).id().clone(), 30)
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
    assert_eq!(validator1_info.weight, 10);
    assert_eq!(validator2_info.weight, 20);
    assert_eq!(validator3_info.weight, 30);

    let total_validator_weight = context.get_total_validator_weight().await?;
    assert_eq!(total_validator_weight, 60);

    context.worker.fast_forward(ONE_EPOCH).await?;

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(95)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(95)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(50)));

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.staked, U128(32500000000000000000000000));
    assert_eq!(validator3_info.staked, U128(47500000000000000000000000));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    // Add a new validator
    context.add_validator(40).await?;

    let validator4_info = context
        .get_validator_info(context.get_stake_pool_contract(3).id().clone())
        .await?;
    assert_eq!(validator4_info.weight, 40);

    let total_validator_weight = context.get_total_validator_weight().await?;
    assert_eq!(total_validator_weight, 100);

    context.worker.fast_forward(ONE_EPOCH).await?;

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(145)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(145)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(50)));

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let validator4_info = context
        .get_validator_info(context.get_stake_pool_contract(3).id().clone())
        .await?;

    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);
    println!("validator 4 info is {:?}", validator4_info);

    assert_eq!(validator1_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.staked, U128(32500000000000000000000000));
    assert_eq!(validator3_info.staked, U128(47500000000000000000000000));
    assert_eq!(validator4_info.staked, U128(50000000000000000000000000));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator4_info.unstaked, U128(0));

    // One more round!
    context.worker.fast_forward(ONE_EPOCH).await?;

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(195)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(195)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(50)));

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(ntoy(195)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(195)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let validator4_info = context
        .get_validator_info(context.get_stake_pool_contract(3).id().clone())
        .await?;

    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);
    println!("validator 4 info is {:?}", validator4_info);

    assert_eq!(validator1_info.staked, U128(19500000000000000000000000));
    assert_eq!(validator2_info.staked, U128(39000000000000000000000000));
    assert_eq!(validator3_info.staked, U128(58500000000000000000000000));
    assert_eq!(validator4_info.staked, U128(78000000000000000000000000));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator4_info.unstaked, U128(0));

    context.worker.fast_forward(ONE_EPOCH).await?;

    // Imbalance the pool by unstaking
    context.unstake(&context.user1, U128(ntoy(30))).await?;
    context.unstake(&context.user2, U128(ntoy(20))).await?;
    context.unstake(&context.user3, U128(ntoy(20))).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(125)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(125)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(70)));

    context.run_epoch_methods().await?;

    let current_epoch = context.get_current_epoch().await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(ntoy(125)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(125)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let validator4_info = context
        .get_validator_info(context.get_stake_pool_contract(3).id().clone())
        .await?;

    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);
    println!("validator 4 info is {:?}", validator4_info);

    assert_eq!(validator1_info.staked, U128(19500000000000000000000000));
    assert_eq!(validator2_info.staked, U128(39000000000000000000000000));
    assert_eq!(validator3_info.staked, U128(58500000000000000000000000));
    assert_eq!(validator4_info.staked, U128(8000000000000000000000000));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator4_info.unstaked, U128(70000000000000000000000000));
    assert_eq!(validator4_info.last_unstake_start_epoch, current_epoch);

    // Update the weights mid way
    context
        .update_validator(context.get_stake_pool_contract(0).id().clone(), 40)
        .await?;
    context
        .update_validator(context.get_stake_pool_contract(1).id().clone(), 30)
        .await?;
    context
        .update_validator(context.get_stake_pool_contract(2).id().clone(), 20)
        .await?;
    context
        .update_validator(context.get_stake_pool_contract(3).id().clone(), 10)
        .await?;

    let total_validator_weight = context.get_total_validator_weight().await?;
    assert_eq!(total_validator_weight, 100);

    context.worker.fast_forward(ONE_EPOCH).await?;

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(ntoy(175)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(175)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(50)));

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(ntoy(175)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(175)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let validator4_info = context
        .get_validator_info(context.get_stake_pool_contract(3).id().clone())
        .await?;

    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);
    println!("validator 4 info is {:?}", validator4_info);

    assert_eq!(validator1_info.staked, U128(69500000000000000000000000));
    assert_eq!(validator2_info.staked, U128(39000000000000000000000000));
    assert_eq!(validator3_info.staked, U128(58500000000000000000000000));
    assert_eq!(validator4_info.staked, U128(8000000000000000000000000));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator4_info.unstaked, U128(70000000000000000000000000));
    assert_eq!(validator4_info.last_unstake_start_epoch, current_epoch);

    Ok(())
}

/// Test ft_on_transfer
#[tokio::test]
async fn test_ft_on_transfer_receiver_failure() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    // get 10 Nearx
    context.deposit(&context.user1, ntoy(10)).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    assert_eq!(user1_account.staked_balance, U128(ntoy(10)));

    context
        .set_stake_pool_panic(context.get_stake_pool_contract(0).id(), true)
        .await?;

    // Transfer 7N to the contract with 3N being used only
    let res = context
        .ft_transfer_call(
            &context.user1,
            context.get_stake_pool_contract(0),
            U128(ntoy(7)),
        )
        .await?;
    println!("res logs are {:?}", res);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    assert_eq!(user1_account.staked_balance, U128(ntoy(10)));

    Ok(())
}

#[tokio::test]
async fn test_ft_on_transfer() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    // get 10 Nearx
    context.deposit(&context.user1, ntoy(10)).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    assert_eq!(user1_account.staked_balance, U128(ntoy(10)));

    context
        .set_refund_amount(U128(ntoy(6)), context.get_stake_pool_contract(0))
        .await?;

    // Transfer 7N to the contract with 3N being used only
    let res = context
        .ft_transfer_call(
            &context.user1,
            context.get_stake_pool_contract(0),
            U128(ntoy(7)),
        )
        .await?;
    println!("res logs are {:?}", res);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    assert_eq!(user1_account.staked_balance, U128(ntoy(9)));

    context
        .set_refund_amount(U128(ntoy(0)), context.get_stake_pool_contract(0))
        .await?;

    let res = context
        .ft_transfer_call(
            &context.user1,
            context.get_stake_pool_contract(0),
            U128(ntoy(9)),
        )
        .await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    assert_eq!(user1_account.staked_balance, U128(ntoy(0)));

    Ok(())
}

/// Stake pool Failures
/// Stake pool deposit_and_stake failures
#[tokio::test]
async fn test_stake_pool_failures_deposit() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;

    println!("user1_depositing 10N");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("user2 depositing 5N");
    context.deposit(&context.user2, ntoy(5)).await?;
    println!("User 3 depositing 2N");
    context.deposit(&context.user3, ntoy(2)).await?;

    context
        .set_stake_pool_panic(context.get_stake_pool_contract(0).id(), true)
        .await?;
    context
        .set_stake_pool_panic(context.get_stake_pool_contract(1).id(), true)
        .await?;
    context
        .set_stake_pool_panic(context.get_stake_pool_contract(2).id(), true)
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(32)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(32)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(17)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(2)),
            withdrawable_epoch: U64(0)
        }
    );

    let res = context.staking_epoch().await?;
    println!("res is {:?}", res.logs());
    // assert!(res.is_err());

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(32)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(32)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(17)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    println!("Getting validator info");
    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(ntoy(5)));
    assert_eq!(validator2_info.staked, U128(ntoy(5)));
    assert_eq!(validator3_info.staked, U128(ntoy(5)));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    println!("Got validator info");

    println!("getting stake pool staked amount");
    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    println!("got stake pool staked amount");

    assert_eq!(stake_pool_1_amount, U128(ntoy(5)));
    assert_eq!(stake_pool_2_amount, U128(ntoy(5)));
    assert_eq!(stake_pool_3_amount, U128(ntoy(5)));

    println!("Getting stake pool unstaked amount");
    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));
    println!("Got stake pool unstaked amount");

    Ok(())
}

#[tokio::test]
async fn test_stake_pool_failures_unstake() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;

    println!("user1_depositing 10N");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("user2 depositing 5N");
    context.deposit(&context.user2, ntoy(5)).await?;
    println!("User 3 depositing 2N");
    context.deposit(&context.user3, ntoy(2)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(32)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(32)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(17)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(2)),
            withdrawable_epoch: U64(0)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(32)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(32)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator2_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator3_info.staked, U128(10666666666666666666666668));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_2_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_3_amount, U128(10666666666666666666666668));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.worker.fast_forward(20 * ONE_EPOCH).await?;

    let current_epoch_2 = context.get_current_epoch().await?;
    println!("current_epoch_2 is {:?}", current_epoch_2);

    context.unstake(&context.user1, U128(ntoy(5))).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(27)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(27)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(5)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(2)),
            withdrawable_epoch: U64(0)
        }
    );

    // set stake pool panic
    context
        .set_stake_pool_panic(context.get_stake_pool_contract(0).id(), true)
        .await?;
    context
        .set_stake_pool_panic(context.get_stake_pool_contract(1).id(), true)
        .await?;
    context
        .set_stake_pool_panic(context.get_stake_pool_contract(2).id(), true)
        .await?;

    context.run_epoch_methods().await?;
    // context.epoch_unstake().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(27)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(27)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(5)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator2_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator3_info.staked, U128(10666666666666666666666668));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_2_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_3_amount, U128(10666666666666666666666668));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    Ok(())
}

#[tokio::test]
async fn test_stake_pool_failures_withdraw() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;

    println!("user1_depositing 10N");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("user2 depositing 5N");
    context.deposit(&context.user2, ntoy(5)).await?;
    println!("User 3 depositing 2N");
    context.deposit(&context.user3, ntoy(2)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(32)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(32)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(17)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(2)),
            withdrawable_epoch: U64(0)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(32)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(32)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator2_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator3_info.staked, U128(10666666666666666666666668));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_2_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_3_amount, U128(10666666666666666666666668));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.worker.fast_forward(ONE_EPOCH).await?;

    let current_epoch_2 = context.get_current_epoch().await?;

    context.unstake(&context.user1, U128(ntoy(5))).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(27)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(27)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(5)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(2)),
            withdrawable_epoch: U64(0)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(27)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(27)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator2_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator3_info.staked, U128(5666666666666666666666668));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(ntoy(5)));
    assert_eq!(validator3_info.last_unstake_start_epoch, current_epoch_2);

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_2_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_3_amount, U128(5666666666666666666666668));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(5)));

    context.worker.fast_forward(ONE_EPOCH * 5).await?;

    let current_epoch_3 = context.get_current_epoch().await?;

    // set stake pool panic
    context
        .set_stake_pool_panic(context.get_stake_pool_contract(0).id(), true)
        .await?;
    context
        .set_stake_pool_panic(context.get_stake_pool_contract(1).id(), true)
        .await?;
    context
        .set_stake_pool_panic(context.get_stake_pool_contract(2).id(), true)
        .await?;

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(27)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(27)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_3);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator2_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator3_info.staked, U128(5666666666666666666666668));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(ntoy(5)));
    assert_eq!(validator3_info.last_unstake_start_epoch, current_epoch_2);

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_2_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_3_amount, U128(5666666666666666666666668));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(5)));

    Ok(())
}

/// User flow specific integration tests
#[tokio::test]
async fn test_eight_epochs_user_flows() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;
    println!("current_epoch_1 is {:?}", current_epoch_1);

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;
    context.unstake(&context.user1, U128(ntoy(5))).await?;
    context.unstake(&context.user2, U128(ntoy(5))).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(35)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(35)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(10)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(35)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(35)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(11666666666666666666666666));
    assert_eq!(validator2_info.staked, U128(11666666666666666666666666));
    assert_eq!(validator3_info.staked, U128(11666666666666666666666668));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(11666666666666666666666666));
    assert_eq!(stake_pool_2_amount, U128(11666666666666666666666666));
    assert_eq!(stake_pool_3_amount, U128(11666666666666666666666668));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.worker.fast_forward(ONE_EPOCH).await?;

    context
        .set_reward_fee(Fraction {
            numerator: 10,
            denominator: 100,
        })
        .await?;
    context.worker.fast_forward(4 * ONE_EPOCH).await?;
    context.commit_reward_fee().await?;

    let reward_fee = context.get_reward_fee().await?;
    assert_eq!(reward_fee.numerator, 10);
    assert_eq!(reward_fee.denominator, 100);

    context.worker.fast_forward(5 * ONE_EPOCH).await?;

    context.deposit(&context.user1, ntoy(5)).await?;
    context.deposit(&context.user2, ntoy(5)).await?;
    context.unstake(&context.user1, U128(ntoy(5))).await?;

    let current_epoch_2 = context.get_current_epoch().await?;

    println!(
        "current_epoch_2_post_reward_fee_set is {:?}",
        current_epoch_2
    );

    context
        .add_stake_pool_rewards(U128(ntoy(10)), context.get_stake_pool_contract(0))
        .await?;
    context
        .add_stake_pool_rewards(U128(ntoy(15)), context.get_stake_pool_contract(1))
        .await?;
    context
        .add_stake_pool_rewards(U128(ntoy(10)), context.get_stake_pool_contract(2))
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(40)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(40)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(10)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(5)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(10)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(75)));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(42298092307692307692307691)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(1773129611955585558323052));

    let nearx_treasury_account = context
        .get_user_account(context.nearx_treasury.id().clone())
        .await?;
    println!("nearx_treasury_account is {:?}", nearx_treasury_account);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(25)));
    assert_eq!(validator2_info.staked, U128(26666666666666666666666666));
    assert_eq!(validator3_info.staked, U128(23333333333333333333333334));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(ntoy(25)));
    assert_eq!(stake_pool_2_amount, U128(26666666666666666666666666));
    assert_eq!(stake_pool_3_amount, U128(23333333333333333333333334));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.worker.fast_forward(ONE_EPOCH).await?;

    let current_epoch_3 = context.get_current_epoch().await?;

    context.unstake(&context.user1, U128(ntoy(5))).await?;
    context.unstake(&context.user3, U128(ntoy(5))).await?;

    context
        .add_stake_pool_rewards(U128(ntoy(10)), context.get_stake_pool_contract(0))
        .await?;
    context
        .add_stake_pool_rewards(U128(ntoy(10)), context.get_stake_pool_contract(1))
        .await?;
    context
        .add_stake_pool_rewards(U128(ntoy(10)), context.get_stake_pool_contract(2))
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(64999999999999999999999998));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(36658346666666666666666665)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(
        nearx_state.user_amount_to_unstake_in_epoch,
        U128(10000000000000000000000002)
    );
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let nearx_price = context.get_nearx_price().await?;
    println!("nearx_price is {:?}", nearx_price);
    assert_eq!(nearx_price, U128(1773129611955585558323052));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(15000000000000000000000001),
            staked_balance: U128(3865648059777927791615260),
            withdrawable_epoch: U64(current_epoch_3.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(17731296119555855583230522),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(5000000000000000000000001),
            staked_balance: U128(12731296119555855583230521),
            withdrawable_epoch: U64(current_epoch_3.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(94999999999999999999999998));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(37979772245333333333333330)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_3);

    let treasury_account = context
        .get_user_account(context.nearx_treasury.id().clone())
        .await?;
    println!("treasury account is {:?}", treasury_account);

    let nearx_price = context.get_nearx_price().await?;
    println!("nearx_price is {:?}", nearx_price);
    assert_eq!(nearx_price, U128(2501331482093678964853995));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(35000000000000000000000000));
    assert_eq!(validator2_info.staked, U128(26666666666666666666666664));
    assert_eq!(validator3_info.staked, U128(33333333333333333333333334));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(10000000000000000000000002));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(35000000000000000000000000));
    assert_eq!(stake_pool_2_amount, U128(26666666666666666666666664));
    assert_eq!(stake_pool_3_amount, U128(33333333333333333333333334));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(0));
    assert_eq!(
        stake_pool_2_unstaked_amount,
        U128(10000000000000000000000002)
    );
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.worker.fast_forward(ONE_EPOCH * 5).await?;

    let contract_balance_before_withdraw = context
        .worker
        .view_account(&context.nearx_contract.id())
        .await?
        .balance;
    context.run_epoch_methods().await?;
    let contract_balance_after_withdraw = context
        .worker
        .view_account(&context.nearx_contract.id())
        .await?
        .balance;
    assert!(abs_diff_eq(
        (contract_balance_after_withdraw - contract_balance_before_withdraw),
        10000000000000000000000002,
        ntoy(1)
    ));

    let user1_balance_before_withdraw = context
        .worker
        .view_account(&context.user1.id())
        .await?
        .balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context
        .worker
        .view_account(&context.user1.id())
        .await?
        .balance;

    assert!(abs_diff_eq(
        (user1_balance_after_withdraw - user1_balance_before_withdraw),
        15000000000000000000000001,
        ntoy(1)
    ));

    let user2_balance_before_withdraw = context
        .worker
        .view_account(&context.user2.id())
        .await?
        .balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context
        .worker
        .view_account(&context.user2.id())
        .await?
        .balance;

    assert!(abs_diff_eq(
        (user2_balance_after_withdraw - user2_balance_before_withdraw),
        ntoy(5),
        ntoy(1)
    ));

    let user3_balance_before_withdraw = context
        .worker
        .view_account(&context.user3.id())
        .await?
        .balance;
    context.withdraw_all(&context.user3).await?;
    let user3_balance_after_withdraw = context
        .worker
        .view_account(&context.user3.id())
        .await?
        .balance;

    assert!(abs_diff_eq(
        (user3_balance_after_withdraw - user3_balance_before_withdraw),
        5000000000000000000000001,
        ntoy(1)
    ));

    Ok(())
}

#[tokio::test]
async fn test_unstake_with_only_private_validators() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;
    println!("current_epoch_1 is {:?}", current_epoch_1);

    context
        .make_validator_private(context.get_stake_pool_contract(0).id(), None)
        .await?;
    context
        .make_validator_private(context.get_stake_pool_contract(1).id(), None)
        .await?;
    context
        .make_validator_private(context.get_stake_pool_contract(2).id(), None)
        .await?;

    println!("user1_depositing 10N");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("user2 depositing 5N");
    context.deposit(&context.user2, ntoy(5)).await?;
    println!("User 3 depositing 2N");
    context.deposit(&context.user3, ntoy(2)).await?;

    context
        .direct_deposit_and_stake(
            &context.user1,
            ntoy(5),
            context.get_stake_pool_contract(0).id(),
        )
        .await?;
    context
        .direct_deposit_and_stake(
            &context.user2,
            ntoy(5),
            context.get_stake_pool_contract(1).id(),
        )
        .await?;
    context
        .direct_deposit_and_stake(
            &context.user3,
            ntoy(6),
            context.get_stake_pool_contract(2).id(),
        )
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(48)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(48)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(17)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(15)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(8)),
            withdrawable_epoch: U64(0)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(48)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(48)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(16)));
    assert_eq!(validator2_info.staked, U128(ntoy(16)));
    assert_eq!(validator3_info.staked, U128(ntoy(16)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(11)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(11)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(10)));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator1_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator2_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator3_info.validator_type, ValidatorType::PRIVATE);

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(ntoy(16)));
    assert_eq!(stake_pool_2_amount, U128(ntoy(16)));
    assert_eq!(stake_pool_3_amount, U128(ntoy(16)));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.unstake(&context.user1, U128(ntoy(12))).await?;
    context.unstake(&context.user2, U128(ntoy(7))).await?;
    context.unstake(&context.user3, U128(ntoy(1))).await?;

    context
        .update_validator_max_unstakable_limit(context.get_stake_pool_contract(0).id(), ntoy(2))
        .await?;
    context
        .update_validator_max_unstakable_limit(context.get_stake_pool_contract(1).id(), ntoy(2))
        .await?;
    context
        .update_validator_max_unstakable_limit(context.get_stake_pool_contract(2).id(), ntoy(1))
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(28)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(28)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(20)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(12)),
            staked_balance: U128(ntoy(3)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(7)),
            staked_balance: U128(ntoy(3)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(1)),
            staked_balance: U128(ntoy(7)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );

    context.worker.fast_forward(ONE_EPOCH).await?;
    let current_epoch_2 = context.get_current_epoch().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(13)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(13)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(11)));
    assert_eq!(validator1_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator2_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator3_info.validator_type, ValidatorType::PRIVATE);

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(28)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(28)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(3)));
    assert_eq!(validator2_info.staked, U128(ntoy(9)));
    assert_eq!(validator3_info.staked, U128(ntoy(16)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(0));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(6)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(11)));
    assert_eq!(validator1_info.unstaked, U128(ntoy(13)));
    assert_eq!(validator2_info.unstaked, U128(ntoy(7)));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(ntoy(3)));
    assert_eq!(stake_pool_2_amount, U128(ntoy(9)));
    assert_eq!(stake_pool_3_amount, U128(ntoy(16)));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(13)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(7)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.worker.fast_forward(ONE_EPOCH * 5).await?;

    let current_epoch_3 = context.get_current_epoch().await?;

    let balance_before_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context.run_epoch_methods().await?;
    let balance_after_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;

    assert!(abs_diff_eq(
        (balance_after_withdraw - balance_before_withdraw),
        ntoy(20),
        ntoy(1)
    ));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(ntoy(3)));
    assert_eq!(validator2_info.staked, U128(ntoy(9)));
    assert_eq!(validator3_info.staked, U128(ntoy(16)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(0));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(6)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(11)));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(0));
    assert_eq!(stake_pool_2_unstaked_amount, U128(0));
    assert_eq!(stake_pool_3_unstaked_amount, U128(0));

    let user1_balance_before_withdraw = context.user1.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context.user1.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user1_balance_after_withdraw - user1_balance_before_withdraw,
        ntoy(12),
        ntoy(1)
    ));

    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user2_balance_after_withdraw - user2_balance_before_withdraw,
        ntoy(7),
        ntoy(1)
    ));

    let user3_balance_before_withdraw = context.user3.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user3).await?;
    let user3_balance_after_withdraw = context.user3.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user3_balance_after_withdraw - user3_balance_before_withdraw,
        ntoy(1),
        ntoy(1)
    ));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user1_account.unstaked_balance, U128(0));
    assert_eq!(user2_account.unstaked_balance, U128(0));
    assert_eq!(user3_account.unstaked_balance, U128(0));

    Ok(())
}

#[tokio::test]
async fn test_unstake_with_private_validators() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;
    println!("current_epoch_1 is {:?}", current_epoch_1);

    context
        .make_validator_private(context.get_stake_pool_contract(0).id(), None)
        .await?;
    context
        .make_validator_private(context.get_stake_pool_contract(1).id(), None)
        .await?;

    println!("user1_depositing 10N");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("user2 depositing 5N");
    context.deposit(&context.user2, ntoy(5)).await?;
    println!("User 3 depositing 2N");
    context.deposit(&context.user3, ntoy(2)).await?;

    context
        .direct_deposit_and_stake(
            &context.user1,
            ntoy(5),
            context.get_stake_pool_contract(0).id(),
        )
        .await?;
    context
        .direct_deposit_and_stake(
            &context.user2,
            ntoy(5),
            context.get_stake_pool_contract(1).id(),
        )
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(42)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(42)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(17)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(15)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(2)),
            withdrawable_epoch: U64(0)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(42)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(42)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(14)));
    assert_eq!(validator2_info.staked, U128(ntoy(14)));
    assert_eq!(validator3_info.staked, U128(ntoy(14)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(9)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(9)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(14)));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator1_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator2_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator3_info.validator_type, ValidatorType::PUBLIC);

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(ntoy(14)));
    assert_eq!(stake_pool_2_amount, U128(ntoy(14)));
    assert_eq!(stake_pool_3_amount, U128(ntoy(14)));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.unstake(&context.user1, U128(ntoy(12))).await?;
    context.unstake(&context.user2, U128(ntoy(7))).await?;
    context.unstake(&context.user3, U128(ntoy(1))).await?;

    context
        .update_validator_max_unstakable_limit(context.get_stake_pool_contract(0).id(), ntoy(2))
        .await?;
    context
        .update_validator_max_unstakable_limit(context.get_stake_pool_contract(1).id(), ntoy(2))
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(22)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(22)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(20)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(12)),
            staked_balance: U128(ntoy(3)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(7)),
            staked_balance: U128(ntoy(3)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(1)),
            staked_balance: U128(ntoy(1)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );

    context.worker.fast_forward(ONE_EPOCH).await?;
    let current_epoch_2 = context.get_current_epoch().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(11)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(11)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(14)));
    assert_eq!(validator1_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator2_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator3_info.validator_type, ValidatorType::PUBLIC);

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(22)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(22)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(8)));
    assert_eq!(validator2_info.staked, U128(ntoy(14)));
    assert_eq!(validator3_info.staked, U128(0));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(5)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(11)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(0)));
    assert_eq!(validator1_info.unstaked, U128(ntoy(6)));
    assert_eq!(validator2_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(14)));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(ntoy(8)));
    assert_eq!(stake_pool_2_amount, U128(ntoy(14)));
    assert_eq!(stake_pool_3_amount, U128(0));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(6)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(14)));

    context.worker.fast_forward(ONE_EPOCH * 5).await?;

    let current_epoch_3 = context.get_current_epoch().await?;

    let balance_before_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context.run_epoch_methods().await?;
    let balance_after_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;

    assert!(abs_diff_eq(
        (balance_after_withdraw - balance_before_withdraw),
        ntoy(20),
        ntoy(1)
    ));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(ntoy(8)));
    assert_eq!(validator2_info.staked, U128(ntoy(14)));
    assert_eq!(validator3_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(0));
    assert_eq!(stake_pool_2_unstaked_amount, U128(0));
    assert_eq!(stake_pool_3_unstaked_amount, U128(0));

    let user1_balance_before_withdraw = context.user1.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context.user1.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user1_balance_after_withdraw - user1_balance_before_withdraw,
        ntoy(12),
        ntoy(1)
    ));

    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user2_balance_after_withdraw - user2_balance_before_withdraw,
        ntoy(7),
        ntoy(1)
    ));

    let user3_balance_before_withdraw = context.user3.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user3).await?;
    let user3_balance_after_withdraw = context.user3.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user3_balance_after_withdraw - user3_balance_before_withdraw,
        ntoy(1),
        ntoy(1)
    ));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user1_account.unstaked_balance, U128(0));
    assert_eq!(user2_account.unstaked_balance, U128(0));
    assert_eq!(user3_account.unstaked_balance, U128(0));

    Ok(())
}

#[tokio::test]
async fn test_unstake_with_private_validators_2() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;
    println!("current_epoch_1 is {:?}", current_epoch_1);

    context
        .make_validator_private(context.get_stake_pool_contract(0).id(), None)
        .await?;
    context
        .make_validator_private(context.get_stake_pool_contract(1).id(), None)
        .await?;

    println!("user1_depositing 10N");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("user2 depositing 5N");
    context.deposit(&context.user2, ntoy(5)).await?;
    println!("User 3 depositing 2N");
    context.deposit(&context.user3, ntoy(2)).await?;

    context
        .direct_deposit_and_stake(
            &context.user1,
            ntoy(5),
            context.get_stake_pool_contract(0).id(),
        )
        .await?;
    context
        .direct_deposit_and_stake(
            &context.user2,
            ntoy(5),
            context.get_stake_pool_contract(1).id(),
        )
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(42)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(42)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(17)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(15)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(2)),
            withdrawable_epoch: U64(0)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(42)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(42)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(14)));
    assert_eq!(validator2_info.staked, U128(ntoy(14)));
    assert_eq!(validator3_info.staked, U128(ntoy(14)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(9)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(9)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(14)));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator1_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator2_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator3_info.validator_type, ValidatorType::PUBLIC);

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(ntoy(14)));
    assert_eq!(stake_pool_2_amount, U128(ntoy(14)));
    assert_eq!(stake_pool_3_amount, U128(ntoy(14)));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.unstake(&context.user1, U128(ntoy(15))).await?;
    context.unstake(&context.user2, U128(ntoy(7))).await?;
    context.unstake(&context.user3, U128(ntoy(1))).await?;
    context
        .unstake(&context.nearx_owner, U128(ntoy(15)))
        .await?;

    context
        .update_validator_max_unstakable_limit(context.get_stake_pool_contract(0).id(), ntoy(2))
        .await?;
    context
        .update_validator_max_unstakable_limit(context.get_stake_pool_contract(1).id(), ntoy(2))
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(4)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(4)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(38)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(15)),
            staked_balance: U128(ntoy(0)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(7)),
            staked_balance: U128(ntoy(3)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(1)),
            staked_balance: U128(ntoy(1)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );

    context.worker.fast_forward(ONE_EPOCH).await?;
    let current_epoch_2 = context.get_current_epoch().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(ntoy(14)));
    assert_eq!(validator2_info.staked, U128(ntoy(14)));
    assert_eq!(validator3_info.staked, U128(ntoy(14)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(11)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(11)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(14)));
    assert_eq!(validator1_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator2_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator3_info.validator_type, ValidatorType::PUBLIC);

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(4)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(4)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(0)));
    assert_eq!(validator2_info.staked, U128(ntoy(4)));
    assert_eq!(validator3_info.staked, U128(0));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(0)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(1)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(0)));
    assert_eq!(validator1_info.unstaked, U128(ntoy(14)));
    assert_eq!(validator2_info.unstaked, U128(ntoy(10)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(14)));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_amount, U128(ntoy(4)));
    assert_eq!(stake_pool_3_amount, U128(ntoy(0)));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(14)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(10)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(14)));

    context.worker.fast_forward(ONE_EPOCH * 5).await?;

    let current_epoch_3 = context.get_current_epoch().await?;

    let balance_before_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context.run_epoch_methods().await?;
    let balance_after_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;

    assert!(abs_diff_eq(
        (balance_after_withdraw - balance_before_withdraw),
        ntoy(38),
        ntoy(1)
    ));

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
    assert_eq!(validator2_info.staked, U128(ntoy(4)));
    assert_eq!(validator3_info.staked, U128(ntoy(0)));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(0));
    assert_eq!(stake_pool_2_unstaked_amount, U128(0));
    assert_eq!(stake_pool_3_unstaked_amount, U128(0));

    let user1_balance_before_withdraw = context.user1.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context.user1.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user1_balance_after_withdraw - user1_balance_before_withdraw,
        ntoy(15),
        ntoy(1)
    ));

    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user2_balance_after_withdraw - user2_balance_before_withdraw,
        ntoy(7),
        ntoy(1)
    ));

    let user3_balance_before_withdraw = context.user3.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user3).await?;
    let user3_balance_after_withdraw = context.user3.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user3_balance_after_withdraw - user3_balance_before_withdraw,
        ntoy(1),
        ntoy(1)
    ));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user1_account.unstaked_balance, U128(0));
    assert_eq!(user2_account.unstaked_balance, U128(0));
    assert_eq!(user3_account.unstaked_balance, U128(0));

    Ok(())
}

#[tokio::test]
async fn test_bank_run_with_private_validators() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;
    println!("current_epoch_1 is {:?}", current_epoch_1);

    context
        .make_validator_private(context.get_stake_pool_contract(0).id(), None)
        .await?;
    context
        .make_validator_private(context.get_stake_pool_contract(1).id(), None)
        .await?;

    // make validator 2 and validator 3 max unstakable limit to 0
    context
        .update_validator_max_unstakable_limit(context.get_stake_pool_contract(0).id(), 0)
        .await?;
    context
        .update_validator_max_unstakable_limit(context.get_stake_pool_contract(1).id(), 0)
        .await?;

    context
        .direct_deposit_and_stake(
            &context.user1,
            ntoy(5),
            context.get_stake_pool_contract(0).id(),
        )
        .await?;
    context
        .direct_deposit_and_stake(
            &context.user2,
            ntoy(5),
            context.get_stake_pool_contract(1).id(),
        )
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

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(10)));
    assert_eq!(validator2_info.staked, U128(ntoy(10)));
    assert_eq!(validator3_info.staked, U128(ntoy(5)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(5)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(5)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(5)));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator1_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator2_info.validator_type, ValidatorType::PRIVATE);
    assert_eq!(validator3_info.validator_type, ValidatorType::PUBLIC);

    println!("user1_depositing 10N");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("user2 depositing 5N");
    context.deposit(&context.user2, ntoy(5)).await?;
    println!("User 3 depositing 2N");
    context.deposit(&context.user3, ntoy(2)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(42)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(42)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(17)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(15)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(2)),
            withdrawable_epoch: U64(0)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(42)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(42)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(14)));
    assert_eq!(validator2_info.staked, U128(ntoy(14)));
    assert_eq!(validator3_info.staked, U128(ntoy(14)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(9)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(9)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(14)));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(ntoy(14)));
    assert_eq!(stake_pool_2_amount, U128(ntoy(14)));
    assert_eq!(stake_pool_3_amount, U128(ntoy(14)));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.unstake(&context.user1, U128(ntoy(15))).await?;
    context.unstake(&context.user2, U128(ntoy(10))).await?;
    context.unstake(&context.user3, U128(ntoy(2))).await?;
    context
        .unstake(&context.nearx_owner, U128(ntoy(15)))
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(0)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(42)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(15)),
            staked_balance: U128(ntoy(0)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(10)),
            staked_balance: U128(ntoy(0)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(2)),
            staked_balance: U128(ntoy(0)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );

    context.worker.fast_forward(ONE_EPOCH).await?;
    let current_epoch_2 = context.get_current_epoch().await?;

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(0)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator2_info.staked, U128(0));
    assert_eq!(validator3_info.staked, U128(0));
    assert_eq!(validator1_info.max_unstakable_limit, U128(0));
    assert_eq!(validator2_info.max_unstakable_limit, U128(0));
    assert_eq!(validator3_info.max_unstakable_limit, U128(0));
    assert_eq!(validator1_info.unstaked, U128(ntoy(14)));
    assert_eq!(validator2_info.unstaked, U128(ntoy(14)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(14)));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(0));
    assert_eq!(stake_pool_2_amount, U128(0));
    assert_eq!(stake_pool_3_amount, U128(0));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(14)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(14)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(14)));

    context.worker.fast_forward(ONE_EPOCH * 5).await?;

    let current_epoch_3 = context.get_current_epoch().await?;

    let balance_before_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context.run_epoch_methods().await?;
    let balance_after_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;

    assert!(abs_diff_eq(
        (balance_after_withdraw - balance_before_withdraw),
        ntoy(42),
        ntoy(1)
    ));

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
    assert_eq!(validator1_info.max_unstakable_limit, U128(0));
    assert_eq!(validator2_info.max_unstakable_limit, U128(0));
    assert_eq!(validator3_info.max_unstakable_limit, U128(0));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    let user1_balance_before_withdraw = context.user1.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context.user1.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user1_balance_after_withdraw - user1_balance_before_withdraw,
        ntoy(15),
        ntoy(1)
    ));

    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user2_balance_after_withdraw - user2_balance_before_withdraw,
        ntoy(10),
        ntoy(1)
    ));

    let user3_balance_before_withdraw = context.user3.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user3).await?;
    let user3_balance_after_withdraw = context.user3.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user3_balance_after_withdraw - user3_balance_before_withdraw,
        ntoy(2),
        ntoy(1)
    ));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user1_account.unstaked_balance, U128(0));
    assert_eq!(user2_account.unstaked_balance, U128(0));
    assert_eq!(user3_account.unstaked_balance, U128(0));

    Ok(())
}

#[tokio::test]
async fn test_bank_run() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;
    println!("current_epoch_1 is {:?}", current_epoch_1);

    println!("user1_depositing 10N");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("user2 depositing 5N");
    context.deposit(&context.user2, ntoy(5)).await?;
    println!("User 3 depositing 2N");
    context.deposit(&context.user3, ntoy(2)).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(32)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(32)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(17)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(5)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(2)),
            withdrawable_epoch: U64(0)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(32)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(32)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator2_info.staked, U128(10666666666666666666666666));
    assert_eq!(validator3_info.staked, U128(10666666666666666666666668));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_2_amount, U128(10666666666666666666666666));
    assert_eq!(stake_pool_3_amount, U128(10666666666666666666666668));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.unstake(&context.user1, U128(ntoy(10))).await?;
    context.unstake(&context.user2, U128(ntoy(5))).await?;
    context.unstake(&context.user3, U128(ntoy(2))).await?;
    context
        .unstake(&context.nearx_owner, U128(ntoy(15)))
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(0)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(32)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(10)),
            staked_balance: U128(ntoy(0)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(0)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(2)),
            staked_balance: U128(ntoy(0)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );

    context.worker.fast_forward(ONE_EPOCH).await?;
    let current_epoch_2 = context.get_current_epoch().await?;

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(0)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator2_info.staked, U128(0));
    assert_eq!(validator3_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(10666666666666666666666666));
    assert_eq!(validator2_info.unstaked, U128(10666666666666666666666666));
    assert_eq!(validator3_info.unstaked, U128(10666666666666666666666668));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(0));
    assert_eq!(stake_pool_2_amount, U128(0));
    assert_eq!(stake_pool_3_amount, U128(0));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(
        stake_pool_1_unstaked_amount,
        U128(10666666666666666666666666)
    );
    assert_eq!(
        stake_pool_2_unstaked_amount,
        U128(10666666666666666666666666)
    );
    assert_eq!(
        stake_pool_3_unstaked_amount,
        U128(10666666666666666666666668)
    );

    context.worker.fast_forward(ONE_EPOCH * 5).await?;

    let current_epoch_3 = context.get_current_epoch().await?;

    let balance_before_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context.run_epoch_methods().await?;
    let balance_after_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;

    assert!(abs_diff_eq(
        (balance_after_withdraw - balance_before_withdraw),
        ntoy(32),
        ntoy(1)
    ));

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
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    let user1_balance_before_withdraw = context.user1.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context.user1.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user1_balance_after_withdraw - user1_balance_before_withdraw,
        ntoy(10),
        ntoy(1)
    ));

    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user2_balance_after_withdraw - user2_balance_before_withdraw,
        ntoy(5),
        ntoy(1)
    ));

    let user3_balance_before_withdraw = context.user3.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user3).await?;
    let user3_balance_after_withdraw = context.user3.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user3_balance_after_withdraw - user3_balance_before_withdraw,
        ntoy(2),
        ntoy(1)
    ));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user1_account.unstaked_balance, U128(0));
    assert_eq!(user2_account.unstaked_balance, U128(0));
    assert_eq!(user3_account.unstaked_balance, U128(0));

    Ok(())
}

#[tokio::test]
async fn test_user_deposit_unstake_autcompounding_withdraw_with_grouped_epoch() -> anyhow::Result<()>
{
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;
    println!("current_epoch_1 is {:?}", current_epoch_1);

    // user 1 deposits 10N
    println!("user1_depositing 10N");
    context.deposit(&context.user1, ntoy(10)).await?;
    // user 2 deposits 5N
    println!("user2 depositing 5N");
    context.deposit(&context.user2, ntoy(5)).await?;
    // user 1 unstakes 5N
    println!("User 1 unstake 5N");
    context.unstake(&context.user1, U128(ntoy(5))).await?;
    // user 3 deposits 2N
    println!("User 3 depositing 2N");
    context.deposit(&context.user3, ntoy(2)).await?;
    // User 2 unstakes 1N
    println!("user 1 unstake 1N");
    context.unstake(&context.user2, U128(ntoy(1))).await?;

    // User 1 transfers 4N to user 3
    println!("User 1 transfers 4N to user 3");
    context
        .ft_transfer(&context.user1, &context.user3, ntoy(4).to_string())
        .await?;

    // User 3 unstakes 4N
    println!("User 3 unstakes 4N");
    context.unstake(&context.user3, U128(ntoy(4))).await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(22)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(22)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(17)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(10)));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(1)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(1)),
            staked_balance: U128(ntoy(4)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(4)),
            staked_balance: U128(ntoy(2)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    // Run user epoch
    println!("Running epoch methods!");
    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(7333333333333333333333333));
    assert_eq!(validator2_info.staked, U128(7333333333333333333333333));
    assert_eq!(validator3_info.staked, U128(7333333333333333333333334));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(7333333333333333333333333));
    assert_eq!(stake_pool_2_amount, U128(7333333333333333333333333));
    assert_eq!(stake_pool_3_amount, U128(7333333333333333333333334));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    // Move one epoch
    context.worker.fast_forward(ONE_EPOCH).await?;
    let current_epoch_2 = context.get_current_epoch().await?;
    println!("current_epoch_2 is {:?}", current_epoch_2);

    context
        .add_stake_pool_rewards(U128(ntoy(11)), &context.get_stake_pool_contract(0))
        .await?;
    context
        .add_stake_pool_rewards(U128(ntoy(10)), &context.get_stake_pool_contract(1))
        .await?;

    println!("user1 depositing");
    context.deposit(&context.user2, ntoy(5)).await?;
    println!("user1 deposited");
    println!("user 1 unstaking ");
    context.unstake(&context.user2, U128(ntoy(6))).await?;
    println!("user1 unstaked");

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(21)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(21)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(6)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(5)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(1)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(7)),
            staked_balance: U128(ntoy(3)),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(4)),
            staked_balance: U128(ntoy(2)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(ntoy(42)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(21)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(17333333333333333333333333));
    assert_eq!(validator2_info.staked, U128(17333333333333333333333333));
    assert_eq!(validator3_info.staked, U128(7333333333333333333333334));
    assert_eq!(validator1_info.unstaked, U128(ntoy(1)));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_2);

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_amount, U128(17333333333333333333333333));
    assert_eq!(stake_pool_2_amount, U128(17333333333333333333333333));
    assert_eq!(stake_pool_3_amount, U128(7333333333333333333333334));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(1)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    context.worker.fast_forward(ONE_EPOCH).await?;
    let current_epoch_3 = context.get_current_epoch().await?;

    // user 1 deposits 10N
    context.deposit(&context.user1, ntoy(10)).await?;
    // user 2 deposits 5N
    context.deposit(&context.user2, ntoy(5)).await?;
    // user 3 deposits 2N
    context.deposit(&context.user3, ntoy(3)).await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(ntoy(60)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(18)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    let owner_account = context
        .get_user_account(context.nearx_owner.id().clone())
        .await?;
    println!("owner_account is {:?}", owner_account);
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(ntoy(12)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(7)),
            staked_balance: U128(ntoy(11)),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(4)),
            staked_balance: U128(ntoy(7)),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    context.run_epoch_methods().await?;
    // while context.epoch_unstake().await?.json::<bool>().unwrap() {};

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(ntoy(60)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_3);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(20000000000000000000000000));
    assert_eq!(validator2_info.staked, U128(20000000000000000000000000));
    assert_eq!(validator3_info.staked, U128(20000000000000000000000000));
    assert_eq!(validator1_info.unstaked, U128(ntoy(1)));
    assert_eq!(validator2_info.unstaked, U128(0));
    assert_eq!(validator3_info.unstaked, U128(0));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_2);

    let stake_pool_1_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_amount = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    println!("stake pool 1 staked amount {:?}", stake_pool_1_amount);
    println!("stake pool 2 staked amount {:?}", stake_pool_2_amount);
    println!("stake pool 3 staked amount {:?}", stake_pool_3_amount);

    assert_eq!(stake_pool_1_amount, U128(20000000000000000000000000));
    assert_eq!(stake_pool_2_amount, U128(20000000000000000000000000));
    assert_eq!(stake_pool_3_amount, U128(20000000000000000000000000));

    let stake_pool_1_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_unstaked_amount = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(2))
        .await?;

    println!(
        "stake pool 1 unstaked amount {:?}",
        stake_pool_1_unstaked_amount
    );
    println!(
        "stake pool 2 unstaked amount {:?}",
        stake_pool_2_unstaked_amount
    );
    println!(
        "stake pool 3 unstaked amount {:?}",
        stake_pool_3_unstaked_amount
    );

    assert_eq!(stake_pool_1_unstaked_amount, U128(ntoy(1)));
    assert_eq!(stake_pool_2_unstaked_amount, U128(ntoy(0)));
    assert_eq!(stake_pool_3_unstaked_amount, U128(ntoy(0)));

    // Test withdraw after 3 epochs
    context.worker.fast_forward(ONE_EPOCH * 10).await?;
    let current_epoch_4 = context.get_current_epoch().await?;
    println!("current_epoch_4 is {:?}", current_epoch_4);

    let balance_before_withdraw = context.nearx_contract.view_account(&context.worker).await?;
    println!(
        "balance_before_withdraw {:?}",
        balance_before_withdraw.balance
    );
    context.run_epoch_methods().await?;
    let balance_after_withdraw = context.nearx_contract.view_account(&context.worker).await?;
    println!(
        "balance_after_withdraw {:?}",
        balance_after_withdraw.balance
    );

    assert!(abs_diff_eq(
        (balance_after_withdraw.balance - balance_before_withdraw.balance),
        ntoy(1),
        900000000000000000000000
    ));

    Ok(())
}

/// Fuzzy integration tests
#[tokio::test]
async fn test_validator_balance_sync() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    println!(
        "stake_pool_1_staked_balance after init is {:?}",
        stake_pool_1_staked_balance
    );

    let current_epoch_1 = context.get_current_epoch().await?;

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
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    context.staking_epoch().await?;

    context
        .adjust_balance(context.get_stake_pool_contract(0).id(), U128(50), U128(50))
        .await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    println!(
        "stake_pool_1_staked_balance is {:?}",
        stake_pool_1_staked_balance
    );
    let stake_pool_1_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    println!(
        "stake_pool_1_unstaked_balance is {:?}",
        stake_pool_1_unstaked_balance
    );
    println!("validator staked is {:?}", validator1_info.staked);
    assert!(abs_diff_eq(
        validator1_info.staked.0,
        stake_pool_1_staked_balance.0,
        50
    ));

    context.worker.fast_forward(1000).await?;

    let current_epoch_2 = context.get_current_epoch().await?;

    context.unstake(&context.user1, U128(ntoy(3))).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    assert_eq!(user1_account.staked_balance, U128(ntoy(7)));
    assert_eq!(user2_account.staked_balance, U128(ntoy(10)));
    assert_eq!(user3_account.staked_balance, U128(ntoy(10)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(42)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(42)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(3)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    context.unstaking_epoch().await?;

    context
        .adjust_balance(context.get_stake_pool_contract(0).id(), U128(50), U128(50))
        .await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;

    let stake_pool_1_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    assert!(abs_diff_eq(
        validator1_info.unstaked.0,
        stake_pool_1_unstaked_balance.0,
        10
    ));

    context
        .sync_validator_balances(context.get_stake_pool_contract(0).id().clone())
        .await?;

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;

    let stake_pool_1_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, stake_pool_1_staked_balance);
    assert_eq!(validator1_info.unstaked, stake_pool_1_unstaked_balance);

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(42)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(42)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    Ok(())
}

#[tokio::test]
async fn test_rebalance_validators() -> anyhow::Result<()> {
    let mut context = IntegrationTestContext::new(3, None).await?;

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
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(15)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(15)));
    assert_eq!(validator2_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(15)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(15)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_staked_balance, U128(ntoy(15)));
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(15)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(15)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    context.worker.fast_forward(ONE_EPOCH * 10).await?;

    let current_epoch_2 = context.get_current_epoch().await?;

    // unstake from val1 and rebalance to val2
    context
        .rebalance_unstake(
            context.get_stake_pool_contract(0).id(),
            context.get_stake_pool_contract(1).id(),
            U128(ntoy(5)),
        )
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
    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(10)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(10)));
    assert_eq!(validator1_info.unstaked, U128(ntoy(5)));
    assert_eq!(
        validator1_info.last_unstake_start_epoch,
        U64(current_epoch_2.0)
    );
    // assert_eq!(validator1_info.redelegate_to, Some(AccountId::from_str(&context.get_stake_pool_contract(1).id().clone().to_string())));
    assert_eq!(validator1_info.amount_to_redelegate, U128(ntoy(5)));
    assert_eq!(validator2_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(15)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(15)));

    context.worker.fast_forward(ONE_EPOCH * 10).await?;

    let current_epoch_3 = context.get_current_epoch().await?;

    // unstake from val1 and rebalance to val2
    let contract_balance_before_rebalance_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context
        .rebalance_withdraw(context.get_stake_pool_contract(0).id())
        .await?;
    let contract_balance_after_rebalance_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;

    assert!(abs_diff_eq(
        contract_balance_after_rebalance_withdraw - contract_balance_before_rebalance_withdraw,
        ntoy(5),
        ntoy(1)
    ));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(10)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(10)));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(
        validator1_info.last_unstake_start_epoch,
        U64(current_epoch_2.0)
    );
    // assert_eq!(validator1_info.redelegate_to, Some(AccountId::from_str(&context.get_stake_pool_contract(1).id().clone().to_string())));
    assert_eq!(validator1_info.amount_to_redelegate, U128(ntoy(5)));
    assert_eq!(validator2_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(15)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(15)));

    context
        .rebalance_stake(context.get_stake_pool_contract(0).id())
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
    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(10)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(10)));
    assert_eq!(validator1_info.unstaked, U128(0));
    assert_eq!(
        validator1_info.last_unstake_start_epoch,
        U64(current_epoch_2.0)
    );
    assert_eq!(validator1_info.redelegate_to, None);
    assert_eq!(validator1_info.amount_to_redelegate, U128(0));
    assert_eq!(validator2_info.staked, U128(ntoy(20)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(20)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(15)));

    Ok(())
}

#[tokio::test]
async fn test_validator_removal() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

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
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(ntoy(15)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(15)));
    assert_eq!(validator2_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(15)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(15)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_staked_balance, U128(ntoy(15)));
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(15)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(15)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    context.worker.fast_forward(ONE_EPOCH * 10).await?;

    let current_epoch_2 = context.get_current_epoch().await?;

    // Pause validator 1
    context
        .pause_validator(&context.get_stake_pool_contract(0).id())
        .await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.weight, 0);

    // drain unstake from validator 1
    println!("Calling drain unstake");
    context
        .drain_unstake(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator1_info.max_unstakable_limit, U128(0));
    assert_eq!(validator1_info.unstaked, U128(ntoy(15)));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_2);

    let stake_pool1_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    assert_eq!(stake_pool1_unstaked_balance, U128(ntoy(15)));

    context.worker.fast_forward(ONE_EPOCH * 10).await?;

    // normal withdraw
    println!("epoch_withdraw");
    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(ntoy(15)));

    // drain withdraw from validator 1
    let contract_balance_before_drain_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    println!("Calling drain withdraw");
    let res = context
        .drain_withdraw(context.get_stake_pool_contract(0).id().clone())
        .await?;
    println!("logs are {:?}", res.failures());
    // println!("res is {:?}", res);
    let contract_balance_after_drain_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;

    assert!(abs_diff_eq(
        contract_balance_after_drain_withdraw - contract_balance_before_drain_withdraw,
        ntoy(15),
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
    assert_eq!(validator1_info.max_unstakable_limit, U128(0));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_2);
    assert_eq!(validator1_info.weight, 0);

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(15)));

    // remove validator 1 from set
    let all_validators = context.get_validators().await?;
    assert_eq!(all_validators.len(), 3);
    println!("Calling remove_validator");
    let res = context
        .remove_validator(&context.get_stake_pool_contract(0).id())
        .await?;
    println!("res logs are {:?}", res.logs());
    let all_validators = context.get_validators().await?;
    assert_eq!(all_validators.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_user_stake_autocompound_unstake_withdraw_flows_all_validators_involved(
) -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

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
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_staked_balance, U128(ntoy(15)));
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(15)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(15)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
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
    assert_eq!(nearx_state.total_staked, U128(ntoy(75)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(75)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(25)));
    assert_eq!(validator2_info.staked, U128(ntoy(25)));
    assert_eq!(validator3_info.staked, U128(ntoy(25)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_staked_balance, U128(ntoy(25)));
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(25)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(25)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(75)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(75)));
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
    assert_eq!(nearx_state.total_staked, U128(ntoy(105)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(105)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(35)));
    assert_eq!(validator2_info.staked, U128(ntoy(35)));
    assert_eq!(validator3_info.staked, U128(ntoy(35)));

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_1_staked_balance, U128(ntoy(35)));
    assert_eq!(stake_pool_2_staked_balance, U128(ntoy(35)));
    assert_eq!(stake_pool_3_staked_balance, U128(ntoy(35)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(105)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(105)));
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
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(75)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_3);

    context.unstaking_epoch().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(40)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_4);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(ntoy(35)));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_4);
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    assert_eq!(validator2_info.staked, U128(ntoy(35)));
    assert_eq!(validator2_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator2_info.last_unstake_start_epoch, U64(0));
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator3_info.staked, U128(ntoy(35)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator3_info.last_unstake_start_epoch, U64(0));

    context.unstaking_epoch().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(5)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_4);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(ntoy(35)));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_4);
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    assert_eq!(validator2_info.staked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(ntoy(35)));
    assert_eq!(validator2_info.last_unstake_start_epoch, current_epoch_4);
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator3_info.staked, U128(ntoy(35)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator3_info.last_unstake_start_epoch, U64(0));

    context.unstaking_epoch().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_4);

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(0));
    assert_eq!(validator1_info.unstaked, U128(ntoy(35)));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch_4);
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    assert_eq!(validator2_info.staked, U128(0));
    assert_eq!(validator2_info.unstaked, U128(ntoy(35)));
    assert_eq!(validator2_info.last_unstake_start_epoch, current_epoch_4);
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator3_info.staked, U128(ntoy(30)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(5)));
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
        .withdraw_epoch(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let contract_after_withdraw_from_val1 = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    assert!(abs_diff_eq(
        contract_after_withdraw_from_val1 - contract_before_withdraw_from_val1,
        ntoy(35),
        ntoy(1)
    ));

    let contract_before_withdraw_from_val2 = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context
        .withdraw_epoch(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let contract_after_withdraw_from_val2 = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    assert!(abs_diff_eq(
        contract_after_withdraw_from_val2 - contract_before_withdraw_from_val2,
        ntoy(35),
        ntoy(1)
    ));

    let contract_before_withdraw_from_val3 = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    context
        .withdraw_epoch(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let contract_after_withdraw_from_val3 = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    assert!(abs_diff_eq(
        contract_after_withdraw_from_val3 - contract_before_withdraw_from_val3,
        ntoy(5),
        ntoy(1)
    ));

    let user1_balance_before_withdraw = context.user1.view_account(&context.worker).await?.balance;
    let res = context.withdraw_all(&context.user1).await?;
    println!("res is {:?}", res);
    let user1_balance_after_withdraw = context.user1.view_account(&context.worker).await?.balance;
    assert!(abs_diff_eq(
        user1_balance_after_withdraw - user1_balance_before_withdraw,
        ntoy(25),
        ntoy(1)
    ));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    assert_eq!(user1_account.unstaked_balance, U128(0));

    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?.balance;
    assert!(abs_diff_eq(
        user2_balance_after_withdraw - user2_balance_before_withdraw,
        ntoy(25),
        ntoy(1)
    ));

    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    assert_eq!(user2_account.unstaked_balance, U128(0));

    Ok(())
}

#[tokio::test]
async fn test_user_stake_autocompound_unstake_withdraw_flows_across_epochs() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

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
    assert_eq!(total_supply, U128(ntoy(56)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(56)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(56)));
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
            staked: U128(ntoy(5)),
            unstaked: U128(0),
            weight: 10,
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            max_unstakable_limit: U128(ntoy(5)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: validator2_info.account_id.clone(),
            staked: U128(ntoy(5)),
            unstaked: U128(0),
            weight: 10,
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            max_unstakable_limit: U128(ntoy(5)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: validator3_info.account_id.clone(),
            staked: U128(ntoy(5)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(5)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    // epoch stake
    while context.staking_epoch().await?.json::<bool>().unwrap() {}

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(18666666666666666666666666));
    assert_eq!(
        validator1_info.max_unstakable_limit,
        U128(18666666666666666666666666)
    );
    assert_eq!(validator2_info.staked, U128(18666666666666666666666666));
    assert_eq!(
        validator2_info.max_unstakable_limit,
        U128(18666666666666666666666666)
    );
    assert_eq!(validator3_info.staked, U128(18666666666666666666666668));
    assert_eq!(
        validator3_info.max_unstakable_limit,
        U128(18666666666666666666666668)
    );

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
        U128(18666666666666666666666666)
    );
    assert_eq!(
        stake_pool_2_staked_balance,
        U128(18666666666666666666666666)
    );
    assert_eq!(
        stake_pool_3_staked_balance,
        U128(18666666666666666666666668)
    );

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(1)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(56)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(56)));
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
        .autocompounding_epoch(context.get_stake_pool_contract(0).id())
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
    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);
    assert_eq!(validator1_info.staked, U128(38666666666666666666666666));
    assert_eq!(
        validator1_info.max_unstakable_limit,
        U128(38666666666666666666666666)
    );
    assert_eq!(validator2_info.staked, U128(18666666666666666666666666));
    assert_eq!(
        validator2_info.max_unstakable_limit,
        U128(18666666666666666666666666)
    );
    assert_eq!(validator3_info.staked, U128(18666666666666666666666668));
    assert_eq!(
        validator3_info.max_unstakable_limit,
        U128(18666666666666666666666668)
    );

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
        U128(38666666666666666666666666)
    );
    assert_eq!(
        stake_pool_2_staked_balance,
        U128(18666666666666666666666666)
    );
    assert_eq!(
        stake_pool_3_staked_balance,
        U128(18666666666666666666666668)
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(106)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(86)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(20)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(ntoy(0)));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(1232558139534883720930232));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: user1_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(30813953488372093023255813),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(30813953488372093023255813),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(ntoy(9)),
            staked_balance: U128(25883720930232558139534883),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    // User 3 unstakes 10N
    context.unstake(&context.user3, U128(ntoy(5))).await?;

    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user3_account is {:?}", user3_account);
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(14000000000000000000000001),
            staked_balance: U128(20883720930232558139534883),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(100999999999999999999999999));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(81943396226415094339622641)
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
    assert_eq!(nearx_price, U128(1232558139534883720930232));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);

    assert_eq!(validator1_info.staked, U128(38666666666666666666666666));
    assert_eq!(validator2_info.staked, U128(18666666666666666666666666));
    assert_eq!(validator3_info.staked, U128(18666666666666666666666668));
    assert_eq!(
        validator1_info.max_unstakable_limit,
        U128(38666666666666666666666666)
    );
    assert_eq!(
        validator2_info.max_unstakable_limit,
        U128(18666666666666666666666666)
    );
    assert_eq!(
        validator3_info.max_unstakable_limit,
        U128(18666666666666666666666668)
    );
    assert_eq!(validator1_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator2_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(0)));

    // epoch unstake
    while context.unstaking_epoch().await?.json::<bool>().unwrap() {}

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(100999999999999999999999999));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(81943396226415094339622641)
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
        U128(38666666666666666666666666)
    );
    assert_eq!(
        stake_pool_2_staked_balance,
        U128(18666666666666666666666666)
    );
    assert_eq!(
        stake_pool_3_staked_balance,
        U128(18666666666666666666666668)
    );

    // fast forward by 1000
    context.worker.fast_forward(1000).await?;
    let current_epoch_2 = context.get_current_epoch().await?;

    // epoch stake
    while context.staking_epoch().await?.json::<bool>().unwrap() {}

    let stake_pool_1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_2_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_3_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    println!(
        "stake_pool_1_staked_balance {:?}",
        stake_pool_1_staked_balance
    );
    println!(
        "stake_pool_2_staked_balance {:?}",
        stake_pool_2_staked_balance
    );
    println!(
        "stake_pool_3_staked_balance {:?}",
        stake_pool_3_staked_balance
    );

    assert_eq!(
        stake_pool_1_staked_balance,
        U128(38666666666666666666666666)
    );
    assert_eq!(
        stake_pool_2_staked_balance,
        U128(33666666666666666666666666)
    );
    assert_eq!(
        stake_pool_3_staked_balance,
        U128(28666666666666666666666667)
    );

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(100999999999999999999999999));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(81943396226415094339622641)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(1232558139534883720930232));

    // epoch autocompound add 10N rewards
    context
        .add_stake_pool_rewards(U128(ntoy(5)), context.get_stake_pool_contract(0))
        .await?;
    context
        .add_stake_pool_rewards(U128(ntoy(5)), context.get_stake_pool_contract(1))
        .await?;

    context
        .autocompounding_epoch(context.get_stake_pool_contract(0).id())
        .await?;
    context
        .autocompounding_epoch(context.get_stake_pool_contract(1).id())
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(110999999999999999999999999));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(81943396226415094339622641)
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
    println!(
        "stake_pool_1_staked_balance {:?}",
        stake_pool_1_staked_balance
    );
    println!(
        "stake_pool_2_staked_balance {:?}",
        stake_pool_2_staked_balance
    );
    println!(
        "stake_pool_3_staked_balance {:?}",
        stake_pool_3_staked_balance
    );

    assert_eq!(
        stake_pool_1_staked_balance,
        U128(43666666666666666666666666)
    );
    assert_eq!(
        stake_pool_2_staked_balance,
        U128(38666666666666666666666666)
    );
    assert_eq!(
        stake_pool_3_staked_balance,
        U128(28666666666666666666666667)
    );

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(43666666666666666666666666));
    assert_eq!(validator2_info.staked, U128(38666666666666666666666666));
    assert_eq!(validator3_info.staked, U128(28666666666666666666666667));
    assert_eq!(
        validator1_info.max_unstakable_limit,
        U128(43666666666666666666666666)
    );
    assert_eq!(
        validator2_info.max_unstakable_limit,
        U128(38666666666666666666666666)
    );
    assert_eq!(
        validator3_info.max_unstakable_limit,
        U128(28666666666666666666666667)
    );
    assert_eq!(validator1_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator2_info.unstaked, U128(ntoy(0)));
    assert_eq!(validator3_info.unstaked, U128(ntoy(0)));

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(1354593598894773198250057));

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
            staked_balance: U128(33864839972369329956251439),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(5)),
            staked_balance: U128(33864839972369329956251439),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(14000000000000000000000001),
            staked_balance: U128(22951416071839742113746257),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );

    // epoch unstake
    while context.unstaking_epoch().await?.json::<bool>().unwrap() {}

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
            .withdraw_epoch(context.get_stake_pool_contract(0).id().clone())
            .await?;
    }
    if validator2_info.unstaked.0 != 0 {
        context
            .withdraw_epoch(context.get_stake_pool_contract(1).id().clone())
            .await?;
    }
    if validator3_info.unstaked.0 != 0 {
        context
            .withdraw_epoch(context.get_stake_pool_contract(2).id().clone())
            .await?;
    }

    // user 1 unstakes 1N
    context.unstake(&context.user1, U128(ntoy(1))).await?;

    // user 2 withdraws
    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?.balance;
    assert!(abs_diff_eq(
        user2_balance_after_withdraw - user2_balance_before_withdraw,
        ntoy(5),
        ntoy(1)
    ));

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
    assert!(abs_diff_eq(
        user3_balance_after_withdraw - user3_balance_before_withdraw,
        ntoy(14),
        ntoy(1)
    ));

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
            staked_balance: U128(32864839972369329956251438),
            withdrawable_epoch: U64(current_epoch_3.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: user2_account.account_id.clone(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(33864839972369329956251438),
            withdrawable_epoch: U64(current_epoch_1.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: user3_account.account_id.clone(),
            unstaked_balance: U128(5000000000000000000000001),
            staked_balance: U128(17951416071839742113746256),
            withdrawable_epoch: U64(current_epoch_3.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    Ok(())
}

/// Happy flows of testing
#[tokio::test]
async fn test_user_stake_unstake_withdraw_flows_in_same_epoch_2() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    // User 1 deposits 10N
    context.deposit(&context.user1, ntoy(10)).await?;
    // User 2 deposits 10N
    context.deposit(&context.user2, ntoy(10)).await?;

    // User 1 unstakes 3 N
    context.unstake(&context.user1, U128(ntoy(3))).await?;
    // User 3 deposits 10N
    context.deposit(&context.user3, ntoy(10)).await?;

    // User 3 unstakes 5N
    context.unstake(&context.user3, U128(ntoy(5))).await?;
    // User 1 deposits 10N
    context.deposit(&context.user1, ntoy(10)).await?;
    // User 2 deposits 10N
    context.deposit(&context.user2, ntoy(10)).await?;
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
    assert_eq!(nearx_state.total_staked, U128(ntoy(37)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(37)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(50)));
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
    assert_eq!(total_token_supply, U128(ntoy(37)));

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(5)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(5)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(5)));

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
            staked: U128(ntoy(5)),
            unstaked: U128(ntoy(0)),
            weight: 10,
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            max_unstakable_limit: U128(ntoy(5)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(ntoy(5)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(5)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(ntoy(5)),
            unstaked: U128(ntoy(0)),
            weight: 10,
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            max_unstakable_limit: U128(ntoy(5)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    // now run epoch stake
    while context.staking_epoch().await?.json::<bool>().unwrap() {}

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;
    assert_eq!(
        stake_pool_contract_balance_0,
        U128(12333333333333333333333333)
    );
    assert_eq!(
        stake_pool_contract_balance_1,
        U128(12333333333333333333333333)
    );
    assert_eq!(
        stake_pool_contract_balance_2,
        U128(12333333333333333333333334)
    );

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(12333333333333333333333333));
    assert_eq!(validator2_info.staked, U128(12333333333333333333333333));
    assert_eq!(validator3_info.staked, U128(12333333333333333333333334));

    // now we run unstake epoch
    while context.unstaking_epoch().await?.json::<bool>().unwrap() {}

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

    assert_eq!(stake_pool_contract_unstaked_balance_0, U128(ntoy(0)));
    assert_eq!(stake_pool_contract_unstaked_balance_1, U128(ntoy(0)));
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

    assert_eq!(
        stake_pool_contract_balance_0,
        U128(12333333333333333333333333)
    );
    assert_eq!(
        stake_pool_contract_balance_1,
        U128(12333333333333333333333333)
    );
    assert_eq!(
        stake_pool_contract_balance_2,
        U128(12333333333333333333333334)
    );

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
            staked: U128(12333333333333333333333333),
            unstaked: U128(ntoy(0)),
            weight: 10,
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            max_unstakable_limit: U128(12333333333333333333333333),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(12333333333333333333333333),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(12333333333333333333333333),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(12333333333333333333333334),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(12333333333333333333333334),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    context.worker.fast_forward(5000).await?;

    println!(
        "epoch_withdraw for val 0 {:?}",
        context.get_stake_pool_contract(0).id()
    );

    if validator1_info.unstaked.0 != 0 {
        context
            .withdraw_epoch(context.get_stake_pool_contract(0).id().clone())
            .await?;
        println!("epoch_withdraw for val 1");
    }

    if validator2_info.unstaked.0 != 0 {
        context
            .withdraw_epoch(context.get_stake_pool_contract(1).id().clone())
            .await?;
    }

    let user1_balance_before_withdraw = context.user1.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context.user1.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user1_balance_after_withdraw - user1_balance_before_withdraw,
        ntoy(13),
        ntoy(1)
    ));

    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user2_balance_after_withdraw - user2_balance_before_withdraw,
        ntoy(10),
        ntoy(1)
    ));

    let user3_balance_before_withdraw = context.user3.view_account(&context.worker).await?.balance;
    context.withdraw_all(&context.user3).await?;
    let user3_balance_after_withdraw = context.user3.view_account(&context.worker).await?.balance;

    assert!(abs_diff_eq(
        user3_balance_after_withdraw - user3_balance_before_withdraw,
        ntoy(5),
        ntoy(1)
    ));

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
    assert_eq!(nearx_state.total_staked, U128(ntoy(67)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(67)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));

    while context.staking_epoch().await?.json::<bool>().unwrap() {}

    let nearx_state = context.get_nearx_state().await?;
    println!(
        "reconcilation epoch is {:?}",
        nearx_state.last_reconcilation_epoch
    );
    let current_epoch = context.get_current_epoch().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch);
    assert_eq!(nearx_state.total_staked, U128(ntoy(67)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(67)));
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

    assert_eq!(
        stake_pool_contract_balance_0,
        U128(22333333333333333333333333)
    );
    assert_eq!(
        stake_pool_contract_balance_1,
        U128(22333333333333333333333333)
    );
    assert_eq!(
        stake_pool_contract_balance_2,
        U128(22333333333333333333333334)
    );

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
            staked: U128(22333333333333333333333333),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(22333333333333333333333333),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(22333333333333333333333333),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(22333333333333333333333333),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(22333333333333333333333334),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(22333333333333333333333334),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    Ok(())
}

#[tokio::test]
async fn test_stake_unstake_and_withdraw_flow_with_reward_boost() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;

    println!("User 1 depositing");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    context.deposit(&context.user2, ntoy(10)).await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    context.deposit(&context.user3, ntoy(10)).await?;
    println!("User 3 successfully deposited");

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: context.user2.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(10)),
            withdrawable_epoch: U64(0)
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

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
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
            staked: U128(ntoy(15)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(ntoy(15)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(ntoy(15)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    context.worker.fast_forward(2 * ONE_EPOCH).await?;

    let current_epoch_2 = context.get_current_epoch().await?;

    // boost rewards
    context
        .update_rewards_buffer(4500000000000000000000000)
        .await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);
    assert_eq!(nearx_state.total_staked, U128(49500000000000000000000000));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(4500000000000000000000000));
    assert_eq!(
        nearx_state.accumulated_rewards_buffer,
        U128(4500000000000000000000000)
    );

    let nearx_price = context.get_nearx_price().await?;
    println!("nearx price is {:?}", nearx_price);
    assert_eq!(nearx_price, U128(1100000000000000000000000));

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(11)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: context.user2.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(11)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: context.user3.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(11)),
            withdrawable_epoch: U64(0)
        }
    );

    // User unstake

    context.unstake(&context.user1, U128(ntoy(5))).await?;
    context.unstake(&context.user2, U128(ntoy(5))).await?;
    context.unstake(&context.user3, U128(ntoy(5))).await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(5000000000000000000000001),
            staked_balance: U128(5999999999999999999999999),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: context.user2.id().clone().parse().unwrap(),
            unstaked_balance: U128(5000000000000000000000001),
            staked_balance: U128(5999999999999999999999999),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: context.user3.id().clone().parse().unwrap(),
            unstaked_balance: U128(5000000000000000000000001),
            staked_balance: U128(5999999999999999999999999),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);
    assert_eq!(nearx_state.total_staked, U128(34499999999999999999999997));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(31363636363636363636363635)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(
        nearx_state.user_amount_to_unstake_in_epoch,
        U128(15000000000000000000000003)
    );
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(4500000000000000000000000));

    // Run this to check the reconcilation given the rewards buffer
    context.staking_epoch().await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);
    assert_eq!(nearx_state.total_staked, U128(34499999999999999999999997));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(31363636363636363636363635)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(
        nearx_state.reconciled_epoch_unstake_amount,
        U128(10500000000000000000000003)
    );
    assert_eq!(nearx_state.rewards_buffer, U128(0));

    context.unstaking_epoch().await?;

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
            staked: U128(4499999999999999999999997),
            unstaked: U128(10500000000000000000000003),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: current_epoch_2,
            weight: 10,
            max_unstakable_limit: U128(4499999999999999999999997),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(ntoy(15)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(ntoy(15)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    context.worker.fast_forward(6 * ONE_EPOCH).await?;

    let current_epoch_3 = context.get_current_epoch().await?;

    let contract_balance_before_withdraw = context
        .worker
        .view_account(&context.nearx_contract.id())
        .await?
        .balance;
    context.run_epoch_methods().await?;
    let contract_balance_after_withdraw = context
        .worker
        .view_account(&context.nearx_contract.id())
        .await?
        .balance;

    assert!(abs_diff_eq(
        (contract_balance_after_withdraw - contract_balance_before_withdraw),
        10500000000000000000000003,
        ntoy(1)
    ));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
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
            staked: U128(4499999999999999999999997),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch_3,
            last_unstake_start_epoch: current_epoch_2,
            weight: 10,
            max_unstakable_limit: U128(4499999999999999999999997),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    context
        .update_rewards_buffer(3450000000000002503999491)
        .await?;

    let nearx_price = context.get_nearx_price().await?;
    println!("nearx price is {:?}", nearx_price);
    assert_eq!(nearx_price, U128(1210000000000000079837664));

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_3);
    assert_eq!(nearx_state.total_staked, U128(37950000000000002503999488));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(31363636363636363636363635)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(3450000000000002503999491));
    assert_eq!(
        nearx_state.accumulated_rewards_buffer,
        U128(7950000000000002503999491)
    );

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    println!("user2_account is {:?}", user2_account);
    println!("user3_account is {:?}", user3_account);

    let owner_account = context
        .get_user_account(context.nearx_owner.id().clone())
        .await?;
    println!("owner_account is {:?}", owner_account);

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(5000000000000000000000001),
            staked_balance: U128(6600000000000000435478171),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: context.user2.id().clone().parse().unwrap(),
            unstaked_balance: U128(5000000000000000000000001),
            staked_balance: U128(6600000000000000435478171),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: context.user3.id().clone().parse().unwrap(),
            unstaked_balance: U128(5000000000000000000000001),
            staked_balance: U128(6600000000000000435478171),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    context.unstake(&context.user1, U128(ntoy(1))).await?;
    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    println!("user1_account is {:?}", user1_account);
    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(6000000000000000000000002),
            staked_balance: U128(5600000000000000435478170),
            withdrawable_epoch: U64(current_epoch_3.0 + NUM_EPOCHS_TO_UNLOCK + 1)
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_3);
    assert_eq!(nearx_state.total_staked, U128(36950000000000002503999487));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(30537190082644628153703751)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(
        nearx_state.user_amount_to_unstake_in_epoch,
        U128(1000000000000000000000001)
    );
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(3450000000000002503999491));

    context.staking_epoch().await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_3);
    assert_eq!(nearx_state.total_staked, U128(36950000000000002503999487));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(30537190082644628153703751)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(
        nearx_state.user_amount_to_unstake_in_epoch,
        U128(1000000000000000000000001)
    );
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(3450000000000002503999491));

    context.worker.fast_forward(2 * ONE_EPOCH).await?;

    let current_epoch_4 = context.get_current_epoch().await?;

    context.staking_epoch().await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_4);
    assert_eq!(nearx_state.total_staked, U128(36950000000000002503999487));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(30537190082644628153703751)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(2450000000000002503999490));
    assert_eq!(
        nearx_state.accumulated_rewards_buffer,
        U128(7950000000000002503999491)
    );

    context.unstaking_epoch().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);
    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(0)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(4499999999999999999999997),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch_3,
            last_unstake_start_epoch: current_epoch_2,
            weight: 10,
            max_unstakable_limit: U128(4499999999999999999999997),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(ntoy(15)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: current_epoch_3,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(ntoy(15)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: current_epoch_3,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(15)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    context.worker.fast_forward(6 * ONE_EPOCH).await?;

    let current_epoch_5 = context.get_current_epoch().await?;

    context.run_epoch_methods().await?;

    let user1_balance_before_withdraw = context
        .worker
        .view_account(&context.user1.id().clone())
        .await?
        .balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context
        .worker
        .view_account(&context.user1.id().clone())
        .await?
        .balance;

    assert!(abs_diff_eq(
        (user1_balance_after_withdraw - user1_balance_before_withdraw),
        6000000000000000000000002,
        ntoy(1)
    ));

    context.worker.fast_forward(3 * ONE_EPOCH).await?;

    let current_epoch_6 = context.get_current_epoch().await?;

    // Add staking rewards
    context
        .add_stake_pool_rewards(U128(ntoy(2)), context.get_stake_pool_contract(0))
        .await?;
    context
        .add_stake_pool_rewards(U128(ntoy(2)), context.get_stake_pool_contract(1))
        .await?;
    context
        .add_stake_pool_rewards(U128(ntoy(2)), context.get_stake_pool_contract(2))
        .await?;

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_6);
    assert_eq!(nearx_state.total_staked, U128(42950000000000002503999487));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(30537190082644628153703751)
    );
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(2450000000000002503999490));
    assert_eq!(
        nearx_state.accumulated_rewards_buffer,
        U128(7950000000000002503999491)
    );

    let nearx_price = context.get_nearx_price().await?;
    println!("nearx_price is {:?}", nearx_price);

    assert_eq!(nearx_price, U128(1406481732070365438079500));

    Ok(())
}

#[tokio::test]
async fn test_bank_run_with_boosted_apr() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;

    context.deposit(&context.user1, ntoy(10)).await?;
    context.deposit(&context.user2, ntoy(10)).await?;
    context.deposit(&context.user3, ntoy(10)).await?;

    context
        .update_rewards_buffer(4500000000000000000000000)
        .await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(11)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: context.user2.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(11)),
            withdrawable_epoch: U64(0)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: context.user3.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(0)),
            staked_balance: U128(ntoy(11)),
            withdrawable_epoch: U64(0)
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));
    assert_eq!(nearx_state.total_staked, U128(49500000000000000000000000));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(4500000000000000000000000));
    assert_eq!(
        nearx_state.accumulated_rewards_buffer,
        U128(4500000000000000000000000)
    );

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);
    assert_eq!(nearx_state.total_staked, U128(49500000000000000000000000));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(4500000000000000000000000));
    assert_eq!(
        nearx_state.accumulated_rewards_buffer,
        U128(4500000000000000000000000)
    );

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);
    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(0)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(16500000000000000000000000),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(16500000000000000000000000),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(16500000000000000000000000),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(16500000000000000000000000),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(12000000000000000000000000),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: current_epoch_1,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(12000000000000000000000000),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    // unstake from all users

    context.worker.fast_forward(2 * ONE_EPOCH).await?;

    let current_epoch_2 = context.get_current_epoch().await?;

    context.unstake(&context.user1, U128(ntoy(11))).await?;
    context.unstake(&context.user2, U128(ntoy(11))).await?;
    context.unstake(&context.user3, U128(ntoy(11))).await?;
    context
        .unstake(&context.nearx_owner, U128(16500000000000000000000000))
        .await?;

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(
        user1_account,
        AccountResponse {
            account_id: context.user1.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(11)),
            staked_balance: U128(ntoy(0)),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user2_account,
        AccountResponse {
            account_id: context.user2.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(11)),
            staked_balance: U128(ntoy(0)),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );
    assert_eq!(
        user3_account,
        AccountResponse {
            account_id: context.user3.id().clone().parse().unwrap(),
            unstaked_balance: U128(ntoy(11)),
            staked_balance: U128(ntoy(0)),
            withdrawable_epoch: U64(current_epoch_2.0 + NUM_EPOCHS_TO_UNLOCK)
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);
    assert_eq!(nearx_state.total_staked, U128(0));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(
        nearx_state.user_amount_to_unstake_in_epoch,
        U128(49500000000000000000000000)
    );
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.rewards_buffer, U128(4500000000000000000000000));
    assert_eq!(
        nearx_state.accumulated_rewards_buffer,
        U128(4500000000000000000000000)
    );

    context.staking_epoch().await?;

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_2);
    assert_eq!(nearx_state.total_staked, U128(0));
    assert_eq!(nearx_state.total_stake_shares, U128(0));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(0));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(
        nearx_state.reconciled_epoch_unstake_amount,
        U128(45000000000000000000000000)
    );
    assert_eq!(nearx_state.rewards_buffer, U128(0));
    assert_eq!(
        nearx_state.accumulated_rewards_buffer,
        U128(4500000000000000000000000)
    );

    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);
    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(0)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(0),
            unstaked: U128(16500000000000000000000000),
            last_asked_rewards_epoch_height: current_epoch_2,
            last_unstake_start_epoch: U64(current_epoch_2.0),
            weight: 10,
            max_unstakable_limit: U128(0),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(0),
            unstaked: U128(16500000000000000000000000),
            last_asked_rewards_epoch_height: current_epoch_2,
            last_unstake_start_epoch: U64(current_epoch_2.0),
            weight: 10,
            max_unstakable_limit: U128(0),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(0),
            unstaked: U128(12000000000000000000000000),
            last_asked_rewards_epoch_height: current_epoch_2,
            last_unstake_start_epoch: U64(current_epoch_2.0),
            weight: 10,
            max_unstakable_limit: U128(0),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    context.worker.fast_forward(7 * ONE_EPOCH).await?;

    let contract_balance_before_withdraw = context
        .worker
        .view_account(&context.nearx_contract.id().clone())
        .await?
        .balance;
    context.run_epoch_methods().await?;
    let contract_balance_after_withdraw = context
        .worker
        .view_account(&context.nearx_contract.id().clone())
        .await?
        .balance;

    assert!(abs_diff_eq(
        (contract_balance_after_withdraw - contract_balance_before_withdraw),
        45000000000000000000000000,
        ntoy(1)
    ));

    let user1_balance_before_withdraw = context
        .worker
        .view_account(&context.user1.id().clone())
        .await?
        .balance;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context
        .worker
        .view_account(&context.user1.id().clone())
        .await?
        .balance;

    assert!(abs_diff_eq(
        (user1_balance_after_withdraw - user1_balance_before_withdraw),
        ntoy(11),
        ntoy(1)
    ));

    let user2_balance_before_withdraw = context
        .worker
        .view_account(&context.user2.id().clone())
        .await?
        .balance;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context
        .worker
        .view_account(&context.user2.id().clone())
        .await?
        .balance;

    assert!(abs_diff_eq(
        (user2_balance_after_withdraw - user2_balance_before_withdraw),
        ntoy(11),
        ntoy(1)
    ));

    let user3_balance_before_withdraw = context
        .worker
        .view_account(&context.user3.id().clone())
        .await?
        .balance;
    context.withdraw_all(&context.user3).await?;
    let user3_balance_after_withdraw = context
        .worker
        .view_account(&context.user3.id().clone())
        .await?
        .balance;

    assert!(abs_diff_eq(
        (user3_balance_after_withdraw - user3_balance_before_withdraw),
        ntoy(11),
        ntoy(1)
    ));

    Ok(())
}

#[tokio::test]
async fn test_user_stake_unstake_withdraw_flows_in_same_epoch() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

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
    assert_eq!(total_token_supply, U128(ntoy(47)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));
    assert_eq!(nearx_state.total_staked, U128(ntoy(47)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(47)));
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
    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(5)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(5)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(5)));

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
            staked: U128(ntoy(5)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(5)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(ntoy(5)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(5)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(ntoy(5)),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(5)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    let current_epoch = context.get_current_epoch().await?;

    // Run stake epoch
    while context.staking_epoch().await?.json::<bool>().unwrap() {}

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
    println!("validator 1 info is {:?}", validator1_info);
    println!("validator 2 info is {:?}", validator2_info);
    println!("validator 3 info is {:?}", validator3_info);

    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: context
                .get_stake_pool_contract(0)
                .id()
                .clone()
                .parse()
                .unwrap(),
            staked: U128(15666666666666666666666666),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(15666666666666666666666666),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(15666666666666666666666666),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(15666666666666666666666666),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(15666666666666666666666668),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(15666666666666666666666668),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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

    assert_eq!(
        stake_pool_contract_balance_0,
        U128(15666666666666666666666666)
    );
    assert_eq!(
        stake_pool_contract_balance_1,
        U128(15666666666666666666666666)
    );
    assert_eq!(
        stake_pool_contract_balance_2,
        U128(15666666666666666666666668)
    );

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
    while context.unstaking_epoch().await?.json::<bool>().unwrap() {}

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
            staked: U128(15666666666666666666666666),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(15666666666666666666666666),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(15666666666666666666666666),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(15666666666666666666666666),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
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
            staked: U128(15666666666666666666666668),
            unstaked: U128(ntoy(0)),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(15666666666666666666666668),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    context.worker.fast_forward(5 * ONE_EPOCH).await?;

    let user1_balance_before_withdraw = context.user1.view_account(&context.worker).await?;
    context.withdraw_all(&context.user1).await?;
    let user1_balance_after_withdraw = context.user1.view_account(&context.worker).await?;

    assert!(abs_diff_eq(
        user1_balance_after_withdraw.balance - user1_balance_before_withdraw.balance,
        ntoy(3),
        ntoy(1)
    ));

    let user2_balance_before_withdraw = context.user2.view_account(&context.worker).await?;
    context.withdraw_all(&context.user2).await?;
    let user2_balance_after_withdraw = context.user2.view_account(&context.worker).await?;

    assert!(abs_diff_eq(
        user2_balance_after_withdraw.balance - user2_balance_before_withdraw.balance,
        ntoy(10),
        ntoy(1)
    ));

    let user3_balance_before_withdraw = context.user3.view_account(&context.worker).await?;
    context.withdraw_all(&context.user3).await?;
    let user3_balance_after_withdraw = context.user3.view_account(&context.worker).await?;

    assert!(abs_diff_eq(
        user3_balance_after_withdraw.balance - user3_balance_before_withdraw.balance,
        ntoy(5),
        ntoy(1)
    ));

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
    let context = IntegrationTestContext::new(3, None).await?;

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
    assert_eq!(validator1_info.staked, U128(ntoy(5)));
    assert_eq!(validator2_info.staked, U128(ntoy(5)));
    assert_eq!(validator3_info.staked, U128(ntoy(5)));

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(5)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(5)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(5)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));

    // Run epoch stake
    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    assert_eq!(validator1_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));

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

    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(15)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(15)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(15)));

    Ok(())
}

// Tests: Unstake with a withdraw following up
#[tokio::test]
async fn test_stake_unstake_and_withdraw_flow_happy_flow() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

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
    assert_eq!(validator1_info.staked, U128(ntoy(5)));
    assert_eq!(validator2_info.staked, U128(ntoy(5)));
    assert_eq!(validator3_info.staked, U128(ntoy(5)));

    let stake_pool_contract_balance_0 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    let stake_pool_contract_balance_1 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(1))
        .await?;
    let stake_pool_contract_balance_2 = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(2))
        .await?;

    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(5)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(5)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(5)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));

    // Run epoch stake
    context.run_epoch_methods().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));

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

    assert_eq!(stake_pool_contract_balance_0, U128(ntoy(15)));
    assert_eq!(stake_pool_contract_balance_1, U128(ntoy(15)));
    assert_eq!(stake_pool_contract_balance_2, U128(ntoy(15)));

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
    assert_eq!(total_token_supply, U128(ntoy(35)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(10)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(35)));
    assert_eq!(nearx_state.total_staked, U128(ntoy(35)));

    context.worker.fast_forward(ONE_EPOCH * 2).await?;

    let validator_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    println!(
        "validator unstaked balance before unstake epoch is {:?}",
        validator_unstaked_balance
    );
    assert_eq!(validator_unstaked_balance, U128(ntoy(0)));

    // Run the unstake epoch
    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(35)));
    assert_eq!(nearx_state.total_staked, U128(ntoy(35)));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));

    let current_epoch = context.get_current_epoch().await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(5)));
    assert_eq!(validator1_info.unstaked, U128(ntoy(10)));
    assert_eq!(validator1_info.last_unstake_start_epoch, current_epoch);

    let validator1_staked_balance = context
        .get_stake_pool_total_staked_amount(context.get_stake_pool_contract(0))
        .await?;
    assert_eq!(validator1_staked_balance, U128(ntoy(5)));

    let validator1_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(0))
        .await?;
    let validator2_unstaked_balance = context
        .get_stake_pool_total_unstaked_amount(context.get_stake_pool_contract(1))
        .await?;
    println!(
        "validator1 unstaked balance after unstake epoch is {:?}",
        validator1_unstaked_balance
    );
    assert_eq!(validator1_unstaked_balance, U128(ntoy(10)));

    // Run the withdraw epoch after 4 epochs to get the amount back
    // Check the contract balance before and after the withdraw call

    context.worker.fast_forward(5 * ONE_EPOCH).await?;

    let balance_before_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    println!("initial contract balance is {:?}", balance_before_withdraw);
    let res = context.run_epoch_methods().await?;
    let balance_after_withdraw = context
        .nearx_contract
        .view_account(&context.worker)
        .await?
        .balance;
    println!("balance after withdraw is {:?}", balance_after_withdraw);
    assert!(abs_diff_eq(
        balance_after_withdraw - balance_before_withdraw,
        ntoy(10),
        ntoy(1)
    ));

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

    assert!(abs_diff_eq(
        user1_balance_after_withdrawal - user1_balance,
        ntoy(5),
        ntoy(1)
    ));

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
    assert!(abs_diff_eq(
        user2_balance_after_withdrawal - user2_balance,
        ntoy(5),
        ntoy(1)
    ));

    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    assert_eq!(user2_account.unstaked_balance, U128(ntoy(0)));

    Ok(())
}

// Tests: Autocompound with treasury rewards and autocompound in the same epoch
#[tokio::test]
async fn test_autocompound_with_treasury_rewards() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new(3, None).await?;

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    assert_eq!(validator1_info.staked, U128(ntoy(5)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(5)));
    assert_eq!(validator2_info.staked, U128(ntoy(5)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(5)));
    assert_eq!(validator3_info.staked, U128(ntoy(5)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(5)));

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

    context.run_epoch_methods().await?;

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
    assert_eq!(validator1_info.staked, U128(ntoy(15)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(15)));
    assert_eq!(validator2_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(15)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(15)));

    let total_token_supply = context.get_total_tokens_supply().await?;
    assert_eq!(total_token_supply, U128(ntoy(45)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(0)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));

    // Set reward fee to 10%
    context
        .set_reward_fee(Fraction {
            numerator: 10,
            denominator: 100,
        })
        .await?;
    context.worker.fast_forward(5 * ONE_EPOCH).await?;
    context.commit_reward_fee().await?;

    let reward_fee = context.get_reward_fee().await?;
    assert_eq!(reward_fee.numerator, 10);
    assert_eq!(reward_fee.denominator, 100);

    context.worker.fast_forward(2 * ONE_EPOCH).await?;

    // Add 30Near of rewards
    context
        .add_stake_pool_rewards(U128(ntoy(45)), context.get_stake_pool_contract(0))
        .await?;

    // Get the operator details

    // auto compound the rewards?;
    context.run_epoch_methods().await?;
    // let res = context
    //     .auto_compound_rewards(context.get_stake_pool_contract(0).id())
    //     .await?;

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(1904761904761904761904761));

    let validator = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    println!("validator is {:?}", validator);
    assert_eq!(
        validator,
        ValidatorInfoResponse {
            account_id: validator.account_id.clone(),
            staked: U128(ntoy(60)),
            unstaked: U128(0),
            weight: 10,
            last_asked_rewards_epoch_height: context.get_current_epoch().await?,
            last_unstake_start_epoch: U64(0),
            max_unstakable_limit: U128(ntoy(60)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(90)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(45)));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(47250000000000000000000000)
    );

    let treasury_account = context
        .get_user_account(context.nearx_treasury.id().clone())
        .await?;
    println!(
        "Treasury account amount is {:?}",
        treasury_account.staked_balance
    );
    assert!(abs_diff_eq(
        treasury_account.staked_balance.0,
        4500000000000000000000000,
        ntoy(1)
    ));

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

    println!("user1_staked_amount is {:?}", user1_staked_amount);
    println!("user2_staked_amount is {:?}", user2_staked_amount);
    println!("user3_staked_amount is {:?}", user3_staked_amount);

    assert_eq!(user1_staked_amount, U128(19047619047619047619047619));
    assert_eq!(user2_staked_amount, U128(19047619047619047619047619));
    assert_eq!(user3_staked_amount, U128(19047619047619047619047619));

    let near_owner_account = context
        .get_user_account(context.nearx_owner.id().clone())
        .await?;

    println!(
        "near_owner_account staked balance is {:?}",
        near_owner_account.staked_balance
    );

    context.worker.fast_forward(1000).await?;
    // Deposit with NearX price > 1
    println!("User 1 depositing");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    context.deposit(&context.user2, ntoy(10)).await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    context.deposit(&context.user3, ntoy(10)).await?;
    println!("User 3 successfully deposited");

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);

    let res = context.run_epoch_methods().await?;
    println!("res is {:?}", res);

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    println!("user1_staked_amount is {:?}", user1_staked_amount);
    println!("user2_staked_amount is {:?}", user2_staked_amount);
    println!("user3_staked_amount is {:?}", user3_staked_amount);

    assert_eq!(user1_staked_amount, U128(29047619047619047619047619));
    assert_eq!(user2_staked_amount, U128(29047619047619047619047619));
    assert_eq!(user3_staked_amount, U128(29047619047619047619047619));

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

    assert_eq!(user1_token_balance, U128(15250000000000000000000000));
    assert_eq!(user2_token_balance, U128(15250000000000000000000000));
    assert_eq!(user3_token_balance, U128(15250000000000000000000000));

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;

    println!("validator1_info is {:?}", validator1_info);
    println!("validator2_info is {:?}", validator2_info);
    println!("validator3_info is {:?}", validator3_info);
    assert_eq!(validator1_info.staked, U128(ntoy(60)));
    assert_eq!(validator1_info.max_unstakable_limit, U128(ntoy(60)));
    assert_eq!(validator2_info.staked, U128(ntoy(40)));
    assert_eq!(validator2_info.max_unstakable_limit, U128(ntoy(40)));
    assert_eq!(validator3_info.staked, U128(ntoy(20)));
    assert_eq!(validator3_info.max_unstakable_limit, U128(ntoy(20)));

    let total_token_supply = context.get_total_tokens_supply().await?;
    assert_eq!(total_token_supply, U128(63000000000000000000000000));

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state is {:?}", nearx_state);
    assert_eq!(nearx_state.total_staked, U128(ntoy(120)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(45)));
    assert_eq!(
        nearx_state.total_stake_shares,
        U128(63000000000000000000000000)
    );

    context
        .autocompounding_epoch(context.get_stake_pool_contract(0).id())
        .await?;

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(1904761904761904761904761));

    Ok(())
}

// Tests: Autocompound with no stake
#[tokio::test]
async fn test_autocompound_with_no_stake() -> anyhow::Result<()> {
    println!("***** Step 1: Initialization *****");
    let context = IntegrationTestContext::new(3, None).await?;

    // Auto compound
    println!("autocompounding!");
    context
        .autocompounding_epoch(context.get_stake_pool_contract(0).id())
        .await?;
    println!("done autocompounding!");

    println!("getting nearx_price");
    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(1)));
    println!("getting nearx_state");
    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(0));
    assert_eq!(nearx_state.total_staked, U128(ntoy(15)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(15)));

    println!("getting validator info");
    let validator_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let current_epoch = context.get_current_epoch().await?;
    assert_eq!(validator_info.staked, U128(ntoy(5)));
    assert_eq!(
        validator_info.last_asked_rewards_epoch_height,
        current_epoch
    );

    Ok(())
}

#[tokio::test]
async fn test_deposit_flows() -> anyhow::Result<()> {
    println!("***** Step 1: Initialization *****");
    let context = IntegrationTestContext::new(3, None).await?;

    let current_epoch_1 = context.get_current_epoch().await?;

    // First test
    // user1, user2 and user3 deposit 10 NEAR each. We check whether the staking contract
    // Check initial deposits
    println!("**** Step 2: User deposit test ****");
    println!("Checking initial user deposits");

    let user1_account = context.get_user_account(context.user1.id().clone()).await?;
    let user2_account = context.get_user_account(context.user2.id().clone()).await?;
    let user3_account = context.get_user_account(context.user3.id().clone()).await?;

    assert_eq!(user1_account.staked_balance, U128(0));
    assert_eq!(user2_account.staked_balance, U128(0));
    assert_eq!(user3_account.staked_balance, U128(0));

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
    assert_eq!(validator1_info.staked, U128(ntoy(5)));
    assert_eq!(validator2_info.staked, U128(ntoy(5)));
    assert_eq!(validator3_info.staked, U128(ntoy(5)));

    println!("**** Simulating user deposits ****");
    println!("User 1 depositing");
    context.deposit(&context.user1, ntoy(10)).await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    context.deposit(&context.user2, ntoy(10)).await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    context.deposit(&context.user3, ntoy(10)).await?;
    println!("User 3 successfully deposited");

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(30)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, U64(0));

    context.run_epoch_methods().await?;

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));
    assert_eq!(nearx_state.user_amount_to_stake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.user_amount_to_unstake_in_epoch, U128(ntoy(0)));
    assert_eq!(nearx_state.reconciled_epoch_stake_amount, U128(0));
    assert_eq!(nearx_state.reconciled_epoch_unstake_amount, U128(0));
    assert_eq!(nearx_state.last_reconcilation_epoch, current_epoch_1);

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
    assert_eq!(validator1_info.staked, U128(ntoy(15)));
    assert_eq!(validator2_info.staked, U128(ntoy(15)));
    assert_eq!(validator3_info.staked, U128(ntoy(15)));

    let nearx_price = context.get_nearx_price().await?;
    println!("nearx_price is {:?}", nearx_price);
    assert_eq!(nearx_price, U128(ntoy(1)));

    let total_staked_amount = context.get_total_staked_amount().await?;
    println!("total_staked_amount is {:?}", total_staked_amount);
    assert_eq!(total_staked_amount, U128(ntoy(45)));

    let total_tokens_minted = context.get_total_tokens_supply().await?;
    assert_eq!(total_tokens_minted, U128(ntoy(45)));

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
    assert_eq!(total_staked_amount, U128(ntoy(45)));

    let total_tokens_minted = context.get_total_tokens_supply().await?;
    assert_eq!(total_tokens_minted, U128(ntoy(45)));

    println!("**** Step 4: Auto compounding ****");

    println!("Fast forward 1 epoch");
    context.worker.fast_forward(ONE_EPOCH).await?;

    println!("Auto compounding stake pool");

    // Adding rewards
    context
        .add_stake_pool_rewards(U128(ntoy(35)), context.get_stake_pool_contract(0))
        .await?;
    context
        .add_stake_pool_rewards(U128(ntoy(5)), context.get_stake_pool_contract(1))
        .await?;
    context
        .add_stake_pool_rewards(U128(ntoy(5)), context.get_stake_pool_contract(2))
        .await?;

    // restake_staking_pool(&worker, &stake_pool_contract).await?;
    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state before auto compounding is {:?}", nearx_state);

    println!("Auto compounding nearx pool");
    let res = context
        .autocompounding_epoch(context.get_stake_pool_contract(0).id())
        .await?;
    println!("res is {:?}", res);
    let res = context
        .autocompounding_epoch(context.get_stake_pool_contract(1).id())
        .await?;
    println!("res is {:?}", res);
    let res = context
        .autocompounding_epoch(context.get_stake_pool_contract(2).id())
        .await?;
    println!("res is {:?}", res);

    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state after auto compounding is {:?}", nearx_state);

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(2)));

    let total_tokens_minted = context.get_total_tokens_supply().await?;
    assert_eq!(total_tokens_minted, U128(ntoy(45)));

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

    let validator1_info = context
        .get_validator_info(context.get_stake_pool_contract(0).id().clone())
        .await?;
    let validator2_info = context
        .get_validator_info(context.get_stake_pool_contract(1).id().clone())
        .await?;
    let validator3_info = context
        .get_validator_info(context.get_stake_pool_contract(2).id().clone())
        .await?;
    let current_epoch = context.get_current_epoch().await?;
    println!("validator1 is {:?}", validator1_info);
    assert_eq!(
        validator1_info,
        ValidatorInfoResponse {
            account_id: validator1_info.account_id.clone(),
            staked: U128(ntoy(50)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(50)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );
    assert_eq!(
        validator2_info,
        ValidatorInfoResponse {
            account_id: validator2_info.account_id.clone(),
            staked: U128(ntoy(20)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(20)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );
    assert_eq!(
        validator3_info,
        ValidatorInfoResponse {
            account_id: validator3_info.account_id.clone(),
            staked: U128(ntoy(20)),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: current_epoch,
            last_unstake_start_epoch: U64(0),
            weight: 10,
            max_unstakable_limit: U128(ntoy(20)),
            validator_type: ValidatorType::PUBLIC,
            redelegate_to: None,
            amount_to_redelegate: U128(0)
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(90)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(45)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));

    println!("nearx_state after auto compounding is {:?}", nearx_state);

    Ok(())
}
