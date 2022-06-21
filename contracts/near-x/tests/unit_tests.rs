mod helpers;

use helpers::ntoy;
use near_sdk::json_types::{U128, U64};
use near_sdk::test_utils::testing_env_with_promise_results;
use near_sdk::{
    testing_env, AccountId, FunctionError, Gas, MockedBlockchain, PromiseOrValue, PromiseResult,
    PublicKey, RuntimeFeesConfig, VMConfig, VMContext,
};
use near_x::constants::NUM_EPOCHS_TO_UNLOCK;
use near_x::contract::{NearxPool, OperationControls};
use near_x::state::{Account, AccountResponse, Fraction, HumanReadableAccount, OperationsControlUpdateRequest, ValidatorInfo, ValidatorInfoResponse};
use std::collections::HashMap;
use std::{convert::TryFrom, str::FromStr};

pub fn owner_account() -> AccountId {
    AccountId::from_str("owner_account").unwrap()
}

pub fn public_key(byte_val: u8) -> PublicKey {
    let mut pk = vec![byte_val; 33];
    pk[0] = 0;
    PublicKey::try_from(pk).unwrap()
}

pub fn system_account() -> AccountId {
    AccountId::from_str("system").unwrap()
}

pub fn to_nanos(num_days: u64) -> u64 {
    num_days * 86_400_000_000_000
}

pub fn to_ts(num_days: u64) -> u64 {
    // 2018-08-01 UTC in nanoseconds
    1_533_081_600_000_000_000 + to_nanos(num_days)
}

pub fn operator_account() -> AccountId {
    AccountId::from_str("operator_account").unwrap()
}

pub fn contract_account() -> AccountId {
    AccountId::from_str("nearx-pool").unwrap()
}

pub fn check_equal_vec<S: PartialEq>(v1: Vec<S>, v2: Vec<S>) -> bool {
    v1.len() == v2.len() && v1.iter().all(|x| v2.contains(x)) && v2.iter().all(|x| v1.contains(x))
}

pub fn default_pubkey() -> PublicKey {
    PublicKey::try_from(vec![0; 33]).unwrap()
}

pub fn get_context(
    predecessor_account_id: AccountId,
    account_balance: u128,
    account_locked_balance: u128,
    block_timestamp: u64,
) -> VMContext {
    VMContext {
        current_account_id: contract_account(),
        signer_account_id: predecessor_account_id.clone(),
        signer_account_pk: default_pubkey(),
        predecessor_account_id,
        input: vec![],
        block_index: 1,
        block_timestamp,
        epoch_height: 1,
        account_balance,
        account_locked_balance,
        storage_usage: 10u64.pow(6),
        attached_deposit: 0,
        prepaid_gas: Gas(10u64.pow(15)), //10u64.pow(15),
        random_seed: [0; 32],
        view_config: None,
        output_data_receivers: vec![],
    }
}

fn get_validator(contract: &NearxPool, validator: AccountId) -> ValidatorInfo {
    contract.validator_info_map.get(&validator).unwrap()
}

fn update_validator(
    contract: &mut NearxPool,
    validator: AccountId,
    validator_info: &ValidatorInfo,
) {
    contract
        .validator_info_map
        .insert(&validator, validator_info)
        .unwrap();
}

fn get_account(contract: &NearxPool, account_id: AccountId) -> Account {
    contract.accounts.get(&account_id).unwrap()
}

fn update_account(contract: &mut NearxPool, account_id: AccountId, account: &Account) {
    contract.accounts.insert(&account_id, account);
}

fn basic_context() -> VMContext {
    get_context(system_account(), ntoy(100), 0, to_ts(500))
}

fn new_contract(owner_account: AccountId, operator_account: AccountId) -> NearxPool {
    NearxPool::new(owner_account, operator_account)
}

fn contract_setup(owner_account: AccountId, operator_account: AccountId) -> (VMContext, NearxPool) {
    let context = basic_context();
    testing_env!(context.clone());
    let contract = new_contract(owner_account, operator_account);
    (context, contract)
}

#[test]
#[should_panic]
fn test_non_owner_calling_update_operations_control() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.predecessor_account_id = operator_account();
    testing_env!(context.clone());

    contract.update_operations_control(OperationsControlUpdateRequest {
        stake_paused: None,
        unstake_paused: None,
        withdraw_paused: None,
        epoch_stake_paused: None,
        epoch_unstake_paused: None,
        epoch_withdraw_paused: None,
        epoch_autocompounding_paused: None,
        sync_validator_balance_paused: None
    });
}

#[test]
fn test_update_operations_control_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.update_operations_control(OperationsControlUpdateRequest {
        stake_paused: Some(true),
        unstake_paused: Some(true),
        withdraw_paused: None,
        epoch_stake_paused: Some(true),
        epoch_unstake_paused: Some(true),
        epoch_withdraw_paused: Some(true),
        epoch_autocompounding_paused: None,
        sync_validator_balance_paused: Some(true)
    });

    let operations_control = contract.get_operations_control();
    assert_eq!(
        operations_control,
        OperationControls {
            stake_paused: true,
            unstaked_paused: true,
            withdraw_paused: false,
            epoch_stake_paused: true,
            epoch_unstake_paused: true,
            epoch_withdraw_paused: true,
            epoch_autocompounding_paused: false,
            sync_validator_balance_paused: true
        }
    );
}

#[test]
#[should_panic]
fn test_add_validator_fail() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
       Non operator adding stake pool
    */
    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone());
}

#[test]
#[should_panic]
fn test_remove_validator_fail() {
    let (_context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
       Non operator removing stake pool
    */
    contract.remove_validator(AccountId::from_str("test_validator").unwrap());
}

#[test]
#[should_panic]
fn test_remove_validator_validator_not_empty() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.predecessor_account_id = owner_account();
    testing_env!(context); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone());

    let mut val1 = get_validator(&contract, stake_public_key_1.clone());
    val1.paused = false;
    val1.staked = ntoy(100);
    update_validator(&mut contract, stake_public_key_1, &val1);

    contract.remove_validator(AccountId::from_str("stake_public_key_1").unwrap());
}

#[test]
fn test_add_validator_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
       initial staking pools should be empty
    */
    let stake_pools = contract.get_validators();
    assert!(
        stake_pools.is_empty(),
        "Stake pools should initially be empty!"
    );

    /*
       add a stake pool
    */
    context.predecessor_account_id = owner_account();
    testing_env!(context); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone());
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_1.clone(),
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }]
    ));

    /*
       add another stake pool
    */
    let stake_public_key_2 = AccountId::from_str("stake_public_key_2").unwrap();

    contract.add_validator(stake_public_key_2.clone());
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 2);
    assert!(check_equal_vec(
        stake_pools,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_1,
                staked: U128(0),
                unstaked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
                paused: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2,
                staked: U128(0),
                unstaked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
                paused: false
            }
        ]
    ));
}

#[test]
fn test_remove_validator_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
       seed staking pools
    */
    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();
    let stake_public_key_2 = AccountId::from_str("stake_public_key_2").unwrap();
    let stake_public_key_3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.predecessor_account_id = owner_account();
    context.epoch_height = 40;
    testing_env!(context); // this updates the context

    contract.add_validator(stake_public_key_1.clone());
    contract.add_validator(stake_public_key_2.clone());
    contract.add_validator(stake_public_key_3.clone());

    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 3);
    assert!(check_equal_vec(
        stake_pools,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_1.clone(),
                staked: U128(0),
                unstaked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
                paused: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(0),
                unstaked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
                paused: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(0),
                unstaked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
                paused: false
            }
        ]
    ));

    /*
       Remove a stake pool
    */
    let mut val1 = get_validator(&contract, stake_public_key_1.clone());
    val1.paused = true;
    val1.unstake_start_epoch = 10;
    update_validator(&mut contract, stake_public_key_1.clone(), &val1);

    contract.remove_validator(stake_public_key_1.clone());
    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 2);
    assert!(check_equal_vec(
        stake_pools,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(0),
                unstaked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
                paused: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(0),
                unstaked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
                paused: false
            }
        ]
    ));

    /*
        Remove another stake pool
    */
    let mut val2 = get_validator(&contract, stake_public_key_2.clone());
    val2.paused = true;
    val1.unstake_start_epoch = 10;
    update_validator(&mut contract, stake_public_key_2.clone(), &val2);

    contract.remove_validator(stake_public_key_2.clone());
    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_3.clone(),
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }]
    ));

    /*
        Remove last stake pool
    */
    let mut val3 = get_validator(&contract, stake_public_key_3.clone());
    val3.paused = true;
    val1.unstake_start_epoch = 10;
    update_validator(&mut contract, stake_public_key_3.clone(), &val3);

    contract.remove_validator(stake_public_key_3);
    let stake_pools = contract.get_validators();

    assert!(stake_pools.is_empty());
}

#[test]
fn test_get_validator_with_min_stake() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
        Get stake pool in empty stake pool set
    */
    let stake_pool = contract.get_validator_to_stake();
    assert!(stake_pool.is_none());

    /*
       seed staking pools
    */
    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();
    let stake_public_key_2 = AccountId::from_str("stake_public_key_2").unwrap();
    let stake_public_key_3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.predecessor_account_id = owner_account();
    testing_env!(context); // this updates the context

    contract.add_validator(stake_public_key_1.clone());
    contract.add_validator(stake_public_key_2.clone());
    contract.add_validator(stake_public_key_3.clone());

    let mut validator_1 = get_validator(&contract, stake_public_key_1.clone());
    let mut validator_2 = get_validator(&contract, stake_public_key_2.clone());
    let mut validator_3 = get_validator(&contract, stake_public_key_3.clone());

    validator_1.staked = 100;
    validator_2.staked = 200;
    validator_3.staked = 300;

    update_validator(&mut contract, stake_public_key_1.clone(), &validator_1);
    update_validator(&mut contract, stake_public_key_2.clone(), &validator_2);
    update_validator(&mut contract, stake_public_key_3.clone(), &validator_3);

    let validators = contract.get_validators();

    assert_eq!(validators.len(), 3);
    assert!(check_equal_vec(
        validators,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_1.clone(),
                staked: U128(100),
                unstaked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
                paused: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(200),
                unstaked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
                paused: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(300),
                unstaked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
                paused: false
            }
        ]
    ));

    /*
       Get stake pool to stake into
    */
    let validator = contract.get_validator_to_stake();
    assert!(validator.is_some());
    assert_eq!(validator.unwrap().account_id, stake_public_key_1);
}

#[test]
#[should_panic]
fn test_set_reward_fee_fail() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
       Set reward fee more than 10%
    */
    contract.set_reward_fee(15, 100);
}

#[test]
#[should_panic]
fn test_deposit_and_stake_direct_stake_contract_busy() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.attached_deposit = 100;
    testing_env!(context);

    contract.contract_lock = true;

    contract.deposit_and_stake_direct_stake();
}

#[test]
#[should_panic]
fn test_deposit_and_stake_direct_stake_min_deposit() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.attached_deposit = 100;
    testing_env!(context);

    contract.min_deposit_amount = 200;

    contract.deposit_and_stake_direct_stake();
}

#[test]
#[should_panic]
fn test_deposit_and_stake_direct_stake_with_no_stake_pools() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.attached_deposit = 5000000000000000000000000;
    testing_env!(context);

    contract.deposit_and_stake_direct_stake();
}

#[test]
fn test_deposit_and_stake_direct_stake_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.predecessor_account_id = owner_account();
    testing_env!(context.clone()); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone());
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_1,
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }]
    ));

    context.attached_deposit = 5000000000000000000000000;
    testing_env!(context);

    contract.deposit_and_stake_direct_stake();
}

#[test]
fn test_stake_pool_deposit_and_stake_direct_stake_callback_fail() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.predecessor_account_id = owner_account();
    testing_env!(context.clone()); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone());
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_1.clone(),
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }]
    ));

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let user = AccountId::from_str("user1").unwrap();

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    validator1.staked = ntoy(10);
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);

    contract.contract_lock = true;

    let res = contract.on_stake_pool_deposit_and_stake_direct(
        validator1,
        ntoy(100),
        ntoy(100),
        AccountId::from_str("user1").unwrap(),
    );
    assert!(matches!(res, PromiseOrValue::Promise(..)));

    assert!(!contract.contract_lock);
}

#[test]
fn test_stake_pool_deposit_and_stake_direct_stake_callback_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.predecessor_account_id = owner_account();
    testing_env!(context.clone()); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone());
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_1.clone(),
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }]
    ));

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    contract.total_stake_shares = ntoy(200);
    contract.total_staked = ntoy(200);

    let user = AccountId::from_str("user1").unwrap();

    testing_env_with_promise_results(context.clone(), PromiseResult::Successful(Vec::default()));
    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);
    contract.contract_lock = true;

    let res = contract.on_stake_pool_deposit_and_stake_direct(
        validator1,
        ntoy(100),
        ntoy(100),
        AccountId::from_str("user1").unwrap(),
    );
    assert!(matches!(res, PromiseOrValue::Value(..)));

    assert_eq!(contract.total_stake_shares, ntoy(300));
    assert_eq!(contract.total_staked, ntoy(300));

    let user_account = contract.get_account(user.clone());
    assert_eq!(
        user_account,
        AccountResponse {
            account_id: user,
            unstaked_balance: U128(0),
            staked_balance: U128(ntoy(100)),
            withdrawable_epoch: U64(0)
        }
    );

    assert!(contract.contract_lock);
}

#[test]
fn test_on_get_sp_staked_balance_reconcile() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.predecessor_account_id = owner_account();
    testing_env!(context.clone()); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone());
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_1.clone(),
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }]
    ));

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let _user = AccountId::from_str("user1").unwrap();

    let res = contract.get_validators();
    println!("res is {:?}", res);
    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    validator1.staked = ntoy(299);
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);

    contract.contract_lock = true;
    contract.total_staked = ntoy(498);

    let _res = contract.on_get_sp_staked_balance_reconcile(
        contract
            .validator_info_map
            .get(&stake_public_key_1)
            .unwrap(),
        ntoy(100),
        U128::from(ntoy(298)),
    );

    let validator1 = get_validator(&contract, stake_public_key_1.clone());
    assert_eq!(validator1.staked, ntoy(298));
    assert_eq!(contract.total_staked, ntoy(497));
    assert!(!contract.contract_lock);
}

#[test]
#[should_panic]
fn test_autocompound_rewards_contract_busy() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.contract_lock = true;

    contract.epoch_autocompound_rewards(AccountId::from_str("random_validator").unwrap());
}

#[test]
#[should_panic]
fn test_autocompound_rewards_invalid_validator() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.epoch_autocompound_rewards(AccountId::from_str("invalid_validator").unwrap());
}

#[test]
#[should_panic]
fn test_autocompound_rewards_validator_busy() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
       Add stake pool
    */
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone()); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone());
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_1.clone(),
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }]
    ));

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    validator1.paused = true;
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);
    contract.epoch_autocompound_rewards(stake_public_key_1);
}

#[test]
fn test_autocompound_rewards_stake_pool_with_no_stake() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
       Add stake pool
    */
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone()); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone());
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_1.clone(),
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }]
    ));

    // Redeeming rewards with no stake amount with validators
    contract.epoch_autocompound_rewards(stake_public_key_1.clone());

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    assert!(!contract.contract_lock);

    /*
       Redeeming rewards in the same epoch
    */

    validator1.last_redeemed_rewards_epoch = context.epoch_height;
    validator1.staked = ntoy(100);

    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);
    contract.epoch_autocompound_rewards(stake_public_key_1.clone());

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    assert!(!contract.contract_lock);

    /*
       Successful case
    */
    context.epoch_height = 100;
    testing_env!(context.clone());
    validator1.last_redeemed_rewards_epoch = context.epoch_height - 10;
    validator1.staked = ntoy(100);
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);

    contract.epoch_autocompound_rewards(stake_public_key_1.clone());

    let validator1 = get_validator(&contract, stake_public_key_1.clone());
    assert!(contract.contract_lock);
}

#[test]
fn test_on_get_sp_staked_balance_for_rewards() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone());
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_1.clone(),
            staked: U128(0),
            unstaked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
            paused: false
        }]
    ));

    context.predecessor_account_id = contract_account();
    context.epoch_height = 100;
    testing_env!(context.clone());

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    validator1.staked = ntoy(100);
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);

    contract.rewards_fee = Fraction::new(10, 100);
    contract.total_staked = ntoy(100);
    contract.total_stake_shares = ntoy(100);

    let _res = contract.on_get_sp_staked_balance_for_rewards(validator1, U128::from(ntoy(150)));

    let validator1 = get_validator(&contract, stake_public_key_1.clone());
    assert!(!contract.contract_lock);
    assert_eq!(validator1.staked, ntoy(150));
    assert_eq!(validator1.last_redeemed_rewards_epoch, context.epoch_height);
    assert_eq!(contract.total_staked, ntoy(150));
    assert_eq!(contract.total_stake_shares, ntoy(100));
    assert_eq!(contract.accumulated_staked_rewards, ntoy(50));
}

#[test]
#[should_panic]
fn test_deposit_and_stake_fail_min_deposit() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.attached_deposit = 100;
    testing_env!(context);

    contract.min_deposit_amount = 200;

    contract.deposit_and_stake();
}

#[test]
#[should_panic]
fn test_deposit_and_stake_fail_zero_amount() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.attached_deposit = 0;
    testing_env!(context);

    contract.deposit_and_stake();
}

#[test]
fn test_deposit_and_stake_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let user1 = AccountId::from_str("user1").unwrap();

    context.attached_deposit = ntoy(100);
    context.predecessor_account_id = user1.clone();
    testing_env!(context);

    contract.min_deposit_amount = ntoy(1);
    contract.total_staked = ntoy(10);
    contract.total_stake_shares = ntoy(10);
    contract.user_amount_to_stake_in_epoch = ntoy(10);

    contract.deposit_and_stake();

    let user1_account = contract.get_account(user1.clone());

    assert_eq!(user1_account.staked_balance, U128(ntoy(100)));
    assert_eq!(contract.total_staked, ntoy(110));
    assert_eq!(contract.total_stake_shares, ntoy(110));
    assert_eq!(contract.user_amount_to_stake_in_epoch, ntoy(110));
}

#[test]
fn test_epoch_reconcilation() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.epoch_height = 100;
    testing_env!(context);

    contract.last_reconcilation_epoch = 99;
    contract.user_amount_to_stake_in_epoch = ntoy(100);
    contract.user_amount_to_unstake_in_epoch = ntoy(150);
    contract.reconciled_epoch_stake_amount = ntoy(10);
    contract.reconciled_epoch_unstake_amount = ntoy(10);

    contract.epoch_reconcilation();

    assert_eq!(contract.user_amount_to_unstake_in_epoch, ntoy(0));
    assert_eq!(contract.user_amount_to_stake_in_epoch, ntoy(0));
    assert_eq!(contract.reconciled_epoch_unstake_amount, ntoy(50));
    assert_eq!(contract.reconciled_epoch_stake_amount, ntoy(0));
    assert_eq!(contract.last_reconcilation_epoch, 100);
}

#[test]
#[should_panic]
fn test_epoch_stake_paused() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.operations_control.epoch_stake_paused = true;

    contract.epoch_stake();
}

#[test]
#[should_panic]
fn test_epoch_unstake_paused() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.operations_control.epoch_unstake_paused = true;

    contract.epoch_unstake();
}

#[test]
#[should_panic]
fn test_epoch_withdraw_paused() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.operations_control.epoch_withdraw_paused = true;

    contract.epoch_withdraw(AccountId::from_str("random_validator").unwrap());
}

#[test]
#[should_panic]
fn test_epoch_autocompounding_paused() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.operations_control.epoch_autocompounding_paused = true;

    contract.epoch_autocompound_rewards(AccountId::from_str("random_validator").unwrap());
}

#[test]
#[should_panic]
fn test_stake_paused() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.operations_control.stake_paused = true;

    contract.deposit_and_stake();
}

#[test]
#[should_panic]
fn test_unstake_paused() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.operations_control.unstaked_paused = true;

    contract.unstake(U128(100));
}

#[test]
#[should_panic]
fn test_withdraw_paused() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.operations_control.withdraw_paused = true;

    contract.withdraw(U128(100));
}

#[test]
fn test_epoch_stake() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context);

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    update_validator(&mut contract, validator1.clone(), &val1_info);

    let mut val2_info = get_validator(&contract, validator2.clone());
    val2_info.staked = ntoy(200);
    update_validator(&mut contract, validator2.clone(), &val2_info);

    contract.last_reconcilation_epoch = 99;
    contract.user_amount_to_stake_in_epoch = ntoy(150);
    contract.user_amount_to_unstake_in_epoch = ntoy(100);
    contract.reconciled_epoch_stake_amount = ntoy(10);
    contract.reconciled_epoch_unstake_amount = ntoy(10);

    contract.epoch_stake();

    assert_eq!(contract.reconciled_epoch_stake_amount, ntoy(0));
    assert_eq!(contract.last_reconcilation_epoch, 100);
}

#[test]
fn test_on_validator_deposit_and_stake_failed() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone());

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.reconciled_epoch_stake_amount = ntoy(10);

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);

    contract.on_stake_pool_deposit_and_stake(validator1.clone(), ntoy(10));

    assert_eq!(contract.reconciled_epoch_stake_amount, ntoy(20));
}

#[test]
fn test_on_validator_deposit_and_stake_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone());

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    update_validator(&mut contract, validator1.clone(), &val1_info);

    testing_env_with_promise_results(context.clone(), PromiseResult::Successful(Vec::default()));

    contract.on_stake_pool_deposit_and_stake(validator1.clone(), ntoy(10));

    let val1_info = get_validator(&contract, validator1.clone());
    assert_eq!(val1_info.staked, ntoy(110));
}

#[test]
fn test_get_unstake_release_epoch() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.predecessor_account_id = owner_account();
    context.epoch_height = 10;
    testing_env!(context.clone());

    // Enough amount available to unstake

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    update_validator(&mut contract, validator1.clone(), &val1_info);

    let mut val2_info = get_validator(&contract, validator2.clone());
    val2_info.staked = ntoy(200);
    update_validator(&mut contract, validator2.clone(), &val2_info);

    let mut val3_info = get_validator(&contract, validator3.clone());
    val3_info.staked = ntoy(300);
    update_validator(&mut contract, validator3.clone(), &val3_info);

    let wait_time = contract.get_unstake_release_epoch(ntoy(100));
    assert_eq!(wait_time, NUM_EPOCHS_TO_UNLOCK);

    context.epoch_height = 10;
    testing_env!(context.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    val1_info.unstake_start_epoch = 9;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    let mut val2_info = get_validator(&contract, validator2.clone());
    val2_info.staked = ntoy(200);
    val2_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator2.clone(), &val2_info);

    let mut val3_info = get_validator(&contract, validator3.clone());
    val3_info.staked = ntoy(300);
    val3_info.unstake_start_epoch = 9;
    update_validator(&mut contract, validator3.clone(), &val3_info);

    let wait_time = contract.get_unstake_release_epoch(ntoy(300));
    assert_eq!(wait_time, 2 * NUM_EPOCHS_TO_UNLOCK);
}

#[test]
#[should_panic]
fn test_withdraw_fail_zero_deposit() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.withdraw(U128(0));
}

#[test]
#[should_panic]
fn test_withdraw_fail_not_enough_amount() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let user1 = AccountId::from_str("user1").unwrap();

    let mut user1_account = Account::default();
    user1_account.unstaked_amount += ntoy(100);
    update_account(&mut contract, user1.clone(), &user1_account);

    context.predecessor_account_id = user1;
    testing_env!(context.clone());

    contract.withdraw(U128(ntoy(200)));
}

#[test]
#[should_panic]
fn test_withdraw_fail_before_withdrawable_epoch() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let user1 = AccountId::from_str("user1").unwrap();

    let mut user1_account = Account::default();
    user1_account.unstaked_amount += ntoy(300);
    user1_account.withdrawable_epoch_height = 10;
    update_account(&mut contract, user1.clone(), &user1_account);

    context.epoch_height = 8;
    context.predecessor_account_id = user1;
    testing_env!(context.clone());

    contract.withdraw(U128(ntoy(200)));
}

#[test]
#[should_panic]
fn test_withdraw_fail_not_enough_storage_balance() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let user1 = AccountId::from_str("user1").unwrap();

    let mut user1_account = Account::default();
    user1_account.unstaked_amount += ntoy(300);
    user1_account.withdrawable_epoch_height = 10;
    update_account(&mut contract, user1.clone(), &user1_account);

    context.epoch_height = 12;
    context.predecessor_account_id = user1;
    context.account_balance = ntoy(230);
    testing_env!(context.clone());

    contract.withdraw(U128(ntoy(200)));
}

#[test]
fn test_withdraw_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let user1 = AccountId::from_str("user1").unwrap();

    let mut user1_account = Account::default();
    user1_account.unstaked_amount += ntoy(300);
    user1_account.withdrawable_epoch_height = 10;
    update_account(&mut contract, user1.clone(), &user1_account);

    context.epoch_height = 12;
    context.predecessor_account_id = user1.clone();
    context.account_balance = ntoy(270);
    testing_env!(context.clone());

    contract.withdraw(U128(ntoy(200)));

    let user1_account = get_account(&contract, user1.clone());
    assert_eq!(user1_account.unstaked_amount, ntoy(100));
}

#[test]
#[should_panic]
fn test_epoch_withdraw_fail_validator_in_unbonding() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.epoch_height = 10;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.unstaked_amount = ntoy(100);
    val1_info.unstake_start_epoch = 9;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.epoch_withdraw(validator1.clone());
}

#[test]
fn test_epoch_withdraw_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.epoch_height = 4;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(50);
    val1_info.unstaked_amount = ntoy(100);
    val1_info.unstake_start_epoch = 9;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.epoch_withdraw(validator1.clone());

    let val1_info = get_validator(&contract, validator1.clone());
    assert_eq!(val1_info.unstaked_amount, ntoy(0));
}

#[test]
fn test_on_stake_pool_withdraw_all_fail() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.epoch_height = 4;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone());

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(50);
    val1_info.unstaked_amount = ntoy(0);
    val1_info.unstake_start_epoch = 9;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);

    contract.on_stake_pool_withdraw_all(val1_info, ntoy(100));

    let val1_info = get_validator(&contract, validator1.clone());
    assert_eq!(val1_info.unstaked_amount, ntoy(100));
}

#[test]
#[should_panic]
fn test_unstake_fail_zero_amount() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.unstake(U128(ntoy(0)));
}

#[test]
#[should_panic]
fn test_unstake_fail_greater_than_total_staked_amount() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.total_staked = ntoy(100);

    contract.unstake(U128(ntoy(200)));
}

#[test]
fn test_unstake_success_diff_epoch_than_reconcilation_epoch() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let user1 = AccountId::from_str("user1").unwrap();
    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    context.epoch_height = 10;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(300);
    val1_info.unstaked_amount = ntoy(0);
    val1_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.total_staked = ntoy(100);
    contract.total_stake_shares = ntoy(100);
    contract.last_reconcilation_epoch = 8;
    contract.user_amount_to_unstake_in_epoch = ntoy(60);

    let mut user1_account = Account::default();
    user1_account.stake_shares = ntoy(50);
    user1_account.unstaked_amount = ntoy(10);
    update_account(&mut contract, user1.clone(), &user1_account);

    context.predecessor_account_id = user1.clone();
    testing_env!(context.clone());

    contract.unstake(U128(ntoy(10)));

    let user1_account = get_account(&contract, user1.clone());
    assert_eq!(user1_account.stake_shares, ntoy(40));
    assert_eq!(user1_account.unstaked_amount, ntoy(20));
    assert_eq!(user1_account.withdrawable_epoch_height, 14);

    assert_eq!(contract.total_staked, ntoy(90));
    assert_eq!(contract.total_stake_shares, ntoy(90));
    assert_eq!(contract.user_amount_to_unstake_in_epoch, ntoy(70));
}

#[test]
fn test_unstake_success_same_epoch_as_reconcilation_epoch() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let user1 = AccountId::from_str("user1").unwrap();
    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    context.epoch_height = 10;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(300);
    val1_info.unstaked_amount = ntoy(0);
    val1_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.total_staked = ntoy(100);
    contract.total_stake_shares = ntoy(100);
    contract.last_reconcilation_epoch = 10;
    contract.user_amount_to_unstake_in_epoch = ntoy(60);

    let mut user1_account = Account::default();
    user1_account.stake_shares = ntoy(50);
    user1_account.unstaked_amount = ntoy(10);
    update_account(&mut contract, user1.clone(), &user1_account);

    context.predecessor_account_id = user1.clone();
    testing_env!(context.clone());

    contract.unstake(U128(ntoy(10)));

    let user1_account = get_account(&contract, user1.clone());
    assert_eq!(user1_account.stake_shares, ntoy(40));
    assert_eq!(user1_account.unstaked_amount, ntoy(20));
    assert_eq!(user1_account.withdrawable_epoch_height, 15);

    assert_eq!(contract.total_staked, ntoy(90));
    assert_eq!(contract.total_stake_shares, ntoy(90));
    assert_eq!(contract.user_amount_to_unstake_in_epoch, ntoy(70));
}

#[test]
fn test_epoch_unstake_fail_less_than_one_near() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 10;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    val1_info.unstaked_amount = ntoy(0);
    val1_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    let mut val2_info = get_validator(&contract, validator2.clone());
    val2_info.staked = ntoy(200);
    val2_info.unstaked_amount = ntoy(0);
    val2_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator2.clone(), &val2_info);

    let mut val3_info = get_validator(&contract, validator3.clone());
    val3_info.staked = ntoy(300);
    val3_info.unstaked_amount = ntoy(0);
    val3_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator3.clone(), &val3_info);

    contract.last_reconcilation_epoch = 99;
    contract.total_staked = ntoy(600);
    contract.user_amount_to_stake_in_epoch = ntoy(1499);
    contract.user_amount_to_unstake_in_epoch = ntoy(1500);
    contract.reconciled_epoch_stake_amount = ntoy(10);
    contract.reconciled_epoch_unstake_amount = ntoy(10);

    let res = contract.epoch_unstake();
    assert_eq!(res, false);
}

#[test]
fn test_epoch_unstake_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    val1_info.unstaked_amount = ntoy(0);
    val1_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    let mut val2_info = get_validator(&contract, validator2.clone());
    val2_info.staked = ntoy(200);
    val2_info.unstaked_amount = ntoy(0);
    val2_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator2.clone(), &val2_info);

    let mut val3_info = get_validator(&contract, validator3.clone());
    val3_info.staked = ntoy(300);
    val3_info.unstaked_amount = ntoy(0);
    val3_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator3.clone(), &val3_info);

    contract.last_reconcilation_epoch = 99;
    contract.user_amount_to_stake_in_epoch = ntoy(100);
    contract.user_amount_to_unstake_in_epoch = ntoy(150);
    contract.reconciled_epoch_stake_amount = ntoy(10);
    contract.reconciled_epoch_unstake_amount = ntoy(10);

    contract.epoch_unstake();

    assert_eq!(contract.last_reconcilation_epoch, 100);
    let val3_info = get_validator(&contract, validator3.clone());
    assert_eq!(val3_info.staked, ntoy(250));
    assert_eq!(val3_info.unstake_start_epoch, 100);
    assert_eq!(val3_info.last_unstake_start_epoch, 3);
    assert_eq!(contract.reconciled_epoch_unstake_amount, 0);
}

#[test]
#[should_panic]
fn test_drain_unstake_fail_validator_not_paused() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    contract.drain_unstake(validator1);
}

#[test]
#[should_panic]
fn test_drain_unstake_fail_validator_pending_release() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.unstake_start_epoch = 99;
    val1_info.paused = true;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.drain_unstake(validator1);
}

#[test]
#[should_panic]
fn test_drain_unstake_fail_validator_has_unstake() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.unstake_start_epoch = 33;
    val1_info.paused = true;
    val1_info.unstaked_amount = ntoy(100);
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.drain_unstake(validator1);
}

#[test]
fn test_drain_unstake_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    val1_info.unstake_start_epoch = 33;
    val1_info.paused = true;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.drain_unstake(validator1.clone());

    let val1_info = get_validator(&contract, validator1.clone());
    assert_eq!(val1_info.staked, ntoy(0));
    assert_eq!(val1_info.unstake_start_epoch, 100);
    assert_eq!(val1_info.last_unstake_start_epoch, 33);
}

#[test]
fn test_on_stake_pool_drain_unstake_promise_fail() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.last_unstake_start_epoch = 33;
    val1.unstake_start_epoch = 100;
    val1.staked = 0;
    update_validator(&mut contract, validator1.clone(), &val1);

    contract.on_stake_pool_drain_unstake(validator1.clone(), ntoy(100));

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 33;
    val1.staked = ntoy(100);
}

#[test]
fn test_on_stake_pool_drain_unstake_promise_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    testing_env_with_promise_results(context.clone(), PromiseResult::Successful(Vec::default()));

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.last_unstake_start_epoch = 33;
    val1.unstake_start_epoch = 100;
    val1.staked = 0;
    val1.unstaked_amount = 0;
    update_validator(&mut contract, validator1.clone(), &val1);

    contract.on_stake_pool_drain_unstake(validator1.clone(), ntoy(100));

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 100;
    val1.staked = 0;
    val1.unstaked_amount = ntoy(100);
}

#[test]
#[should_panic]
fn test_drain_withdraw_fail_validator_not_paused() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    contract.drain_withdraw(validator1);
}

#[test]
#[should_panic]
fn test_drain_withdraw_fail_validator_has_non_zero_staked() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 100;
    val1.staked = ntoy(100);
    val1.unstaked_amount = 0;
    val1.paused = true;
    update_validator(&mut contract, validator1.clone(), &val1);

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    contract.drain_withdraw(validator1);
}

#[test]
#[should_panic]
fn test_drain_withdraw_fail_validator_pending_unstake() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 99;
    val1.staked = ntoy(0);
    val1.unstaked_amount = ntoy(100);
    val1.paused = true;
    update_validator(&mut contract, validator1.clone(), &val1);

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    contract.drain_withdraw(validator1);
}

#[test]
fn test_drain_withdraw_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 23;
    val1.staked = ntoy(0);
    val1.unstaked_amount = ntoy(100);
    val1.paused = true;
    update_validator(&mut contract, validator1.clone(), &val1);

    contract.drain_withdraw(validator1.clone());

    let mut val1 = get_validator(&contract, validator1.clone());
    assert_eq!(val1.unstaked_amount, 0);
}

#[test]
fn test_on_stake_pool_drain_withdraw_failure() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 88;
    val1.staked = ntoy(0);
    val1.unstaked_amount = ntoy(0);
    val1.paused = true;
    update_validator(&mut contract, validator1.clone(), &val1);

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);

    contract.on_stake_pool_drain_withdraw(validator1.clone(), ntoy(100));

    let mut val1 = get_validator(&contract, validator1.clone());
    assert_eq!(val1.unstaked_amount, ntoy(100));
}

#[test]
fn test_on_stake_pool_drain_withdraw_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    contract.user_amount_to_stake_in_epoch = ntoy(100);

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 88;
    val1.staked = ntoy(0);
    val1.unstaked_amount = ntoy(0);
    val1.paused = true;
    update_validator(&mut contract, validator1.clone(), &val1);

    testing_env_with_promise_results(context.clone(), PromiseResult::Successful(Vec::default()));

    contract.on_stake_pool_drain_withdraw(validator1.clone(), ntoy(100));

    assert_eq!(contract.user_amount_to_stake_in_epoch, ntoy(200));
}

#[test]
#[should_panic]
fn test_sync_balance_from_validator_paused() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.operations_control.sync_validator_balance_paused = true;

    contract.sync_balance_from_validator(AccountId::from_str("abc").unwrap());
}

#[test]
fn test_sync_balance_from_validator_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());


    contract.sync_balance_from_validator(validator1);
}

#[test]
fn test_on_stake_pool_get_account() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    testing_env!(context.clone());

    contract.add_validator(validator1.clone());
    contract.add_validator(validator2.clone());
    contract.add_validator(validator3.clone());

    let mut validator1_info = get_validator(&contract, validator1.clone());
    validator1_info.staked = ntoy(99);
    validator1_info.unstaked_amount = ntoy(9);
    update_validator(&mut contract, validator1.clone(), &validator1_info);

    contract.on_stake_pool_get_account(validator1.clone(), HumanReadableAccount {
        account_id: validator1.clone(),
        unstaked_balance: U128(ntoy(10)),
        staked_balance: U128(ntoy(100)),
        can_withdraw: false
    });

    let mut validator1_info = get_validator(&contract, validator1.clone());
    assert_eq!(validator1_info.staked, ntoy(100));
    assert_eq!(validator1_info.unstaked_amount, ntoy(10));
}