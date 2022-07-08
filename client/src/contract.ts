import * as nearjs from 'near-api-js';
import { Epoch, SnapshotUser, ValidatorInfo } from '.';
import { nameof } from './utils';

export type NearxContract = nearjs.Contract &
  RpcCallsStakingPool &
  RpcCallsOperator &
  RpcCallsUtils;

/**
 * The parameters used for every RPC call to the contract.
 */
export interface RpcParams {
  /** The gas the caller is willing to pay for the transaction. */
  gas?: string;
  /** The deposit joined to the call. */
  amount?: string;
  /** The contract arguments. */
  [name: string]: any;
}

export interface RpcCallsStakingPool {
  get_account_staked_balance(params: RpcParams): Promise<string>;
  get_account_total_balance(params: RpcParams): Promise<string>;
  deposit(params: RpcParams): Promise<string>;
  deposit_and_stake_direct_stake(params: RpcParams): Promise<string>;
  deposit_and_stake(params: RpcParams): Promise<string>;
  stake(params: RpcParams): Promise<string>;
  withdraw(params: RpcParams): Promise<string>;
  withdraw_all(params: RpcParams): Promise<string>;
  unstake(params: RpcParams): Promise<string>;
  unstake_all(params: RpcParams): Promise<string>;
}

export interface RpcCallsOperator {
  'new'(params: RpcParams): Promise<ValidatorInfo[]>;

  get_validators(params: RpcParams): Promise<ValidatorInfo[]>;
  get_number_of_accounts(params: RpcParams): Promise<number>;
  get_snapshot_users(params: RpcParams): Promise<SnapshotUser[]>;

  epoch_stake(params: RpcParams): Promise<string>;
  epoch_autocompound_rewards(params: RpcParams): Promise<string>;
  epoch_unstake(params: RpcParams): Promise<string>;
  epoch_withdraw(params: RpcParams): Promise<string>;
  sync_balance_from_validator(params: RpcParams): Promise<string>;
  upgrade(code: any, gas: any): Promise<string>;
}

export interface RpcCallsUtils {
  get_current_epoch(params: RpcParams): Promise<Epoch>;
}

export function createContract(account: nearjs.Account, contractName: string): NearxContract {
  return new nearjs.Contract(
    // The account object that is connecting:
    account,

    // Name of contract you're connecting to:
    contractName,

    // Options:
    {
      viewMethods: [
        // Staking Pool:
        nameof<RpcCallsStakingPool>('get_account_staked_balance'),
        nameof<RpcCallsStakingPool>('get_account_total_balance'),
        // Operator:
        nameof<RpcCallsOperator>('get_validators'),
        nameof<RpcCallsOperator>('get_number_of_accounts'),
        nameof<RpcCallsOperator>('get_snapshot_users'),
        // Utils:
        nameof<RpcCallsUtils>('get_current_epoch'),
      ],
      changeMethods: [
        nameof<RpcCallsOperator>('new'),
        // Staking Pool:
        nameof<RpcCallsStakingPool>('deposit'),
        nameof<RpcCallsStakingPool>('deposit_and_stake_direct_stake'),
        nameof<RpcCallsStakingPool>('deposit_and_stake'),
        nameof<RpcCallsStakingPool>('stake'),
        nameof<RpcCallsStakingPool>('withdraw'),
        nameof<RpcCallsStakingPool>('withdraw_all'),
        nameof<RpcCallsStakingPool>('unstake'),
        nameof<RpcCallsStakingPool>('unstake_all'),
        // Operator:
        nameof<RpcCallsOperator>('epoch_stake'),
        nameof<RpcCallsOperator>('epoch_autocompound_rewards'),
        nameof<RpcCallsOperator>('epoch_unstake'),
        nameof<RpcCallsOperator>('epoch_withdraw'),
        nameof<RpcCallsOperator>('sync_balance_from_validator'),
        nameof<RpcCallsOperator>('upgrade'),
      ],
      //sender: account, // account object to initialize and sign transactions.
    },
  ) as NearxContract;
}
