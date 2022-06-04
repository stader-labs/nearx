mod helpers;

use helpers::ntoy;
use near_sdk::json_types::{U128, U64};
use near_sdk::test_utils::testing_env_with_promise_results;
use near_sdk::{
    testing_env, AccountId, Gas, MockedBlockchain, PromiseOrValue, PromiseResult, PublicKey,
    RuntimeFeesConfig, VMConfig, VMContext,
};
use std::collections::HashMap;
use std::{convert::TryFrom, str::FromStr};
use near_x::contract::NearxPool;
use near_x::state::{AccountResponse, Fraction, ValidatorInfo, ValidatorInfoResponse};

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
fn test_add_staking_pool_fail() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
       Non operator adding stake pool
    */
    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone());
}

#[test]
#[should_panic]
fn test_remove_staking_pool_fail() {
    let (_context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
       Non operator removing stake pool
    */
    contract.remove_validator(AccountId::from_str("test_validator").unwrap());
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
            last_asked_rewards_epoch_height: U64(0),
            lock: false
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
                last_asked_rewards_epoch_height: U64(0),
                lock: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2,
                staked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                lock: false
            }
        ]
    ));
}

#[test]
fn test_remove_staking_pool_success() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

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

    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 3);
    assert!(check_equal_vec(
        stake_pools,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_1.clone(),
                staked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                lock: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                lock: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                lock: false
            }
        ]
    ));

    /*
       Remove a stake pool
    */
    contract.remove_validator(stake_public_key_1.clone());
    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 2);
    assert!(check_equal_vec(
        stake_pools,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                lock: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(0),
                last_asked_rewards_epoch_height: U64(0),
                lock: false
            }
        ]
    ));

    /*
        Remove another stake pool
    */
    contract.remove_validator(stake_public_key_2.clone());
    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_3.clone(),
            staked: U128(0),
            last_asked_rewards_epoch_height: U64(0),
            lock: false
        }]
    ));

    /*
        Remove last stake pool
    */
    contract.remove_validator(stake_public_key_3);
    let stake_pools = contract.get_validators();

    assert!(stake_pools.is_empty());
}

#[test]
fn test_get_stake_pool_with_min_stake() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
        Get stake pool in empty stake pool set
    */
    let stake_pool = contract.get_stake_pool_with_min_stake();
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
                last_asked_rewards_epoch_height: U64(0),
                lock: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(200),
                last_asked_rewards_epoch_height: U64(0),
                lock: false
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(300),
                last_asked_rewards_epoch_height: U64(0),
                lock: false
            }
        ]
    ));

    /*
       Get stake pool to stake into
    */
    let validator = contract.get_stake_pool_with_min_stake();
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
fn test_deposit_and_stake_contract_busy() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.attached_deposit = 100;
    testing_env!(context);

    contract.contract_lock = true;

    contract.deposit_and_stake();
}

#[test]
#[should_panic]
fn test_deposit_and_stake_min_deposit() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.attached_deposit = 100;
    testing_env!(context);

    contract.min_deposit_amount = 200;

    contract.deposit_and_stake();
}

#[test]
#[should_panic]
fn test_deposit_and_stake_with_no_stake_pools() {
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

    context.attached_deposit = 5000000000000000000000000;
    testing_env!(context);

    contract.deposit_and_stake();
}

#[test]
fn test_deposit_and_stake_success() {
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
            last_asked_rewards_epoch_height: U64(0),
            lock: false
        }]
    ));

    context.attached_deposit = 5000000000000000000000000;
    testing_env!(context);

    contract.deposit_and_stake();
}

#[test]
fn test_stake_pool_deposit_and_stake_callback_fail() {
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
            last_asked_rewards_epoch_height: U64(0),
            lock: false
        }]
    ));

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let user = AccountId::from_str("user1").unwrap();

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    validator1.lock = true;
    validator1.staked = ntoy(10);
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);

    contract.contract_lock = true;

    let res =
        contract.on_stake_pool_deposit_and_stake(validator1, ntoy(100), ntoy(100), user.clone());
    assert!(matches!(res, PromiseOrValue::Promise(..)));

    assert!(!contract.contract_lock);

    let validator1 = get_validator(&contract, stake_public_key_1.clone());
    assert!(!validator1.lock);
}

#[test]
fn test_stake_pool_deposit_and_stake_callback_success() {
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
            last_asked_rewards_epoch_height: U64(0),
            lock: false
        }]
    ));

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    contract.total_stake_shares = ntoy(200);
    contract.total_staked = ntoy(200);

    let user = AccountId::from_str("user1").unwrap();

    testing_env_with_promise_results(context.clone(), PromiseResult::Successful(Vec::default()));
    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    validator1.lock = true;
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);
    contract.contract_lock = true;

    let res =
        contract.on_stake_pool_deposit_and_stake(validator1, ntoy(100), ntoy(100), user.clone());
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
            can_withdraw: false
        }
    );

    let validator1 = get_validator(&contract, stake_public_key_1.clone());
    assert!(validator1.lock);
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
            last_asked_rewards_epoch_height: U64(0),
            lock: false
        }]
    ));

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let _user = AccountId::from_str("user1").unwrap();

    let res = contract.get_validators();
    println!("res is {:?}", res);
    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    validator1.lock = true;
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
    assert!(!validator1.lock);
    assert!(!contract.contract_lock);
}

#[test]
#[should_panic]
fn test_autocompound_rewards_contract_busy() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.contract_lock = true;

    contract.autocompound_rewards(AccountId::from_str("random_validator").unwrap());
}

#[test]
#[should_panic]
fn test_autocompound_rewards_invalid_stake_pool() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.autocompound_rewards(AccountId::from_str("invalid_stake_pool").unwrap());
}

#[test]
#[should_panic]
fn test_autocompound_rewards_stake_pool_busy() {
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
            last_asked_rewards_epoch_height: U64(0),
            lock: false
        }]
    ));

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    validator1.lock = true;
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);
    contract.autocompound_rewards(stake_public_key_1);
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
            last_asked_rewards_epoch_height: U64(0),
            lock: false
        }]
    ));

    // Redeeming rewards with no stake amount with validators
    contract.autocompound_rewards(stake_public_key_1.clone());

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    assert!(!contract.contract_lock);
    assert!(!validator1.lock);

    /*
       Redeeming rewards in the same epoch
    */

    validator1.last_redeemed_rewards_epoch = context.epoch_height;
    validator1.staked = ntoy(100);

    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);
    contract.autocompound_rewards(stake_public_key_1.clone());

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    assert!(!contract.contract_lock);
    assert!(!validator1.lock);

    /*
       Successful case
    */
    context.epoch_height = 100;
    testing_env!(context.clone());
    validator1.last_redeemed_rewards_epoch = context.epoch_height - 10;
    validator1.staked = ntoy(100);
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);

    contract.autocompound_rewards(stake_public_key_1.clone());

    let validator1 = get_validator(&contract, stake_public_key_1.clone());
    assert!(contract.contract_lock);
    assert!(validator1.lock);
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
            last_asked_rewards_epoch_height: U64(0),
            lock: false
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
    assert!(!validator1.lock);
    assert_eq!(validator1.staked, ntoy(150));
    assert_eq!(validator1.last_redeemed_rewards_epoch, context.epoch_height);
    assert_eq!(contract.total_staked, ntoy(150));
    assert_eq!(contract.total_stake_shares, ntoy(100));
    assert_eq!(contract.accumulated_staked_rewards, ntoy(50));
}
