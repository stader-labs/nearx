mod helpers;

use helpers::ntoy;
use near_liquid_token::{
    types::{AccountResponse, U128String, U64String, ValidatorInfoResponse},
    NearxPool,
};
use near_sdk::{
    testing_env, AccountId, Gas, MockedBlockchain, PromiseOrValue, PromiseResult, PublicKey,
    RuntimeFeesConfig, VMConfig, VMContext,
};
use near_sdk_sim::{to_ts, utils::system_account};
use std::{convert::TryFrom, str::FromStr};

pub fn owner_account() -> AccountId {
    AccountId::from_str("owner_account").unwrap()
}

pub fn public_key(byte_val: u8) -> PublicKey {
    let mut pk = vec![byte_val; 33];
    pk[0] = 0;
    PublicKey::try_from(pk).unwrap()
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

pub fn testing_env_with_promise_results(context: VMContext, promise_result: PromiseResult) {
    let storage = near_sdk::env::take_blockchain_interface()
        .unwrap()
        .as_mut_mocked_blockchain()
        .unwrap()
        .take_storage();

    near_sdk::env::set_blockchain_interface(MockedBlockchain::new(
        context,
        VMConfig::test(),
        RuntimeFeesConfig::test(),
        vec![promise_result],
        storage,
        Default::default(),
        Default::default(),
    ));
}

pub fn default_pubkey() -> PublicKey {
    PublicKey::try_from(vec![0]).unwrap()
}

pub fn get_context(
    predecessor_account_id: AccountId,
    account_balance: u128,
    account_locked_balance: u128,
    block_timestamp: u64,
    //view_config: bool,
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

fn basic_context() -> VMContext {
    let account = AccountId::from_str(&system_account()).unwrap();

    get_context(account, ntoy(100), 0, to_ts(500))
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
    let (mut context, mut contract) = contract_setup(owner_account(), operator_account());

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
    contract.remove_validator(0);
}

#[test]
fn test_add_staking_pool_success() {
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
            inx: 0,
            account_id: stake_public_key_1,
            staked: U128String::from(0),
            last_asked_rewards_epoch_height: U64String::from(0),
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
                inx: 0,
                account_id: stake_public_key_1,
                staked: U128String::from(0),
                last_asked_rewards_epoch_height: U64String::from(0),
                lock: false
            },
            ValidatorInfoResponse {
                inx: 1,
                account_id: stake_public_key_2,
                staked: U128String::from(0),
                last_asked_rewards_epoch_height: U64String::from(0),
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
                inx: 0,
                account_id: stake_public_key_1.clone(),
                staked: U128String::from(0),
                last_asked_rewards_epoch_height: U64String::from(0),
                lock: false
            },
            ValidatorInfoResponse {
                inx: 1,
                account_id: stake_public_key_2.clone(),
                staked: U128String::from(0),
                last_asked_rewards_epoch_height: U64String::from(0),
                lock: false
            },
            ValidatorInfoResponse {
                inx: 2,
                account_id: stake_public_key_3.clone(),
                staked: U128String::from(0),
                last_asked_rewards_epoch_height: U64String::from(0),
                lock: false
            }
        ]
    ));

    /*
       Remove a stake pool
    */
    contract.remove_validator(0);
    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 2);
    assert!(check_equal_vec(
        stake_pools,
        vec![
            ValidatorInfoResponse {
                inx: 0,
                account_id: stake_public_key_2.clone(),
                staked: U128String::from(0),
                last_asked_rewards_epoch_height: U64String::from(0),
                lock: false
            },
            ValidatorInfoResponse {
                inx: 1,
                account_id: stake_public_key_3.clone(),
                staked: U128String::from(0),
                last_asked_rewards_epoch_height: U64String::from(0),
                lock: false
            }
        ]
    ));

    /*
        Remove another stake pool
    */
    contract.remove_validator(0);
    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            inx: 0,
            account_id: stake_public_key_3.clone(),
            staked: U128String::from(0),
            last_asked_rewards_epoch_height: U64String::from(0),
            lock: false
        }]
    ));

    /*
        Remove last stake pool
    */
    contract.remove_validator(0);
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

    // Set the stake amount
    contract.validators[0].staked = 100;
    contract.validators[1].staked = 200;
    contract.validators[2].staked = 300;

    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 3);
    assert!(check_equal_vec(
        stake_pools,
        vec![
            ValidatorInfoResponse {
                inx: 0,
                account_id: stake_public_key_1.clone(),
                staked: U128String::from(100),
                last_asked_rewards_epoch_height: U64String::from(0),
                lock: false
            },
            ValidatorInfoResponse {
                inx: 1,
                account_id: stake_public_key_2.clone(),
                staked: U128String::from(200),
                last_asked_rewards_epoch_height: U64String::from(0),
                lock: false
            },
            ValidatorInfoResponse {
                inx: 2,
                account_id: stake_public_key_3.clone(),
                staked: U128String::from(300),
                last_asked_rewards_epoch_height: U64String::from(0),
                lock: false
            }
        ]
    ));

    /*
       Get stake pool to stake into
    */
    let stake_pool = contract.get_stake_pool_with_min_stake();
    assert!(stake_pool.is_some());
    let stake_pool_inx = stake_pool.unwrap();
    assert_eq!(stake_pool_inx, 0);
}

#[test]
#[should_panic]
fn test_set_reward_fee_fail() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    /*
       Set reward fee more than 10%
    */
    contract.set_reward_fee(15);
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
            inx: 0,
            account_id: stake_public_key_1,
            staked: U128String::from(0),
            last_asked_rewards_epoch_height: U64String::from(0),
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
            inx: 0,
            account_id: stake_public_key_1,
            staked: U128String::from(0),
            last_asked_rewards_epoch_height: U64String::from(0),
            lock: false
        }]
    ));

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let user = AccountId::from_str("user1").unwrap();

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);
    contract.validators[0].lock = true;
    contract.contract_lock = true;

    let res = contract.on_stake_pool_deposit_and_stake(0, ntoy(100), ntoy(100), user.clone());
    assert!(matches!(res, PromiseOrValue::Promise(..)));

    assert!(!contract.validators[0].lock);
    assert!(!contract.contract_lock);
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
            inx: 0,
            account_id: stake_public_key_1,
            staked: U128String::from(0),
            last_asked_rewards_epoch_height: U64String::from(0),
            lock: false
        }]
    ));

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    contract.total_stake_shares = ntoy(200);
    contract.total_staked = ntoy(200);

    let user = AccountId::from_str("user1").unwrap();

    testing_env_with_promise_results(context.clone(), PromiseResult::Successful(Vec::default()));
    contract.validators[0].lock = true;
    contract.contract_lock = true;

    let res = contract.on_stake_pool_deposit_and_stake(0, ntoy(100), ntoy(100), user.clone());
    assert!(matches!(res, PromiseOrValue::Value(..)));

    assert_eq!(contract.total_stake_shares, ntoy(300));
    assert_eq!(contract.total_staked, ntoy(300));

    let user_account = contract.get_account(user.clone());
    assert_eq!(
        user_account,
        AccountResponse {
            account_id: user,
            unstaked_balance: U128String::from(0),
            staked_balance: U128String::from(ntoy(100)),
            can_withdraw: false
        }
    );

    assert!(contract.validators[0].lock);
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
            inx: 0,
            account_id: stake_public_key_1,
            staked: U128String::from(0),
            last_asked_rewards_epoch_height: U64String::from(0),
            lock: false
        }]
    ));

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let user = AccountId::from_str("user1").unwrap();

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);
    contract.validators[0].lock = true;
    contract.contract_lock = true;
    contract.validators[0].staked = ntoy(299);
    contract.total_staked = ntoy(498);

    let res =
        contract.on_get_sp_staked_balance_reconcile(0, ntoy(100), U128String::from(ntoy(298)));
    assert_eq!(contract.validators[0].staked, ntoy(298));
    assert_eq!(contract.total_staked, ntoy(497));
    assert!(!contract.validators[0].lock);
    assert!(!contract.contract_lock);
}

#[test]
#[should_panic]
fn test_distribute_rewards_contract_busy() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.contract_lock = true;

    contract.distribute_rewards(0);
}

#[test]
#[should_panic]
fn test_distribute_rewards_invalid_stake_pool() {
    let (mut _context, mut contract) = contract_setup(owner_account(), operator_account());

    contract.distribute_rewards(0);
}

#[test]
#[should_panic]
fn test_distribute_rewards_stake_pool_busy() {
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
            inx: 0,
            account_id: stake_public_key_1,
            staked: U128String::from(0),
            last_asked_rewards_epoch_height: U64String::from(0),
            lock: false
        }]
    ));

    contract.validators[0].lock = true;
    contract.distribute_rewards(0);
}

#[test]
fn test_distribute_rewards_stake_pool_with_no_stake() {
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
            inx: 0,
            account_id: stake_public_key_1,
            staked: U128String::from(0),
            last_asked_rewards_epoch_height: U64String::from(0),
            lock: false
        }]
    ));

    // Redeeming rewards with no stake amount with validators
    contract.distribute_rewards(0);

    assert!(!contract.contract_lock);
    assert!(!contract.validators[0].lock);

    /*
       Redeeming rewards in the same epoch
    */
    contract.validators[0].last_redeemed_rewards_epoch = context.epoch_height;
    contract.validators[0].staked = ntoy(100);

    contract.distribute_rewards(0);

    assert!(!contract.contract_lock);
    assert!(!contract.validators[0].lock);

    /*
       Successful case
    */
    context.epoch_height = 100;
    testing_env!(context.clone());
    contract.validators[0].last_redeemed_rewards_epoch = context.epoch_height - 10;
    contract.validators[0].staked = ntoy(100);

    contract.distribute_rewards(0);

    assert!(contract.contract_lock);
    assert!(contract.validators[0].lock);
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
            inx: 0,
            account_id: stake_public_key_1,
            staked: U128String::from(0),
            last_asked_rewards_epoch_height: U64String::from(0),
            lock: false
        }]
    ));

    context.predecessor_account_id = contract_account();
    context.epoch_height = 100;
    testing_env!(context.clone());

    contract.validators[0].staked = ntoy(100);
    contract.rewards_fee_pct = 10;
    contract.total_staked = ntoy(100);
    contract.total_stake_shares = ntoy(100);

    let res = contract.on_get_sp_staked_balance_for_rewards(0, U128String::from(ntoy(150)));

    assert!(!contract.contract_lock);
    assert!(!contract.validators[0].lock);
    assert_eq!(contract.validators[0].staked, ntoy(150));
    assert_eq!(
        contract.validators[0].last_redeemed_rewards_epoch,
        context.epoch_height
    );
    assert_eq!(contract.total_staked, ntoy(150));
    assert_eq!(contract.total_stake_shares, ntoy(100));
    assert_eq!(contract.accumulated_staked_rewards, ntoy(50));
}
