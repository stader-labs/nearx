import * as nearjs from 'near-api-js';
import { Epoch, NearxAccount, SnapshotUser, ValidatorInfo } from '.';
import { nameof } from './utils';

export type NearxContract = nearjs.Contract &
  RpcCallsStakingPool &
  RpcCallsOperator &
  RpcCallsUtils &
  RpcCallsFt;

/**
 * The parameters used for every RPC call to the contract.
 */
export interface CallRpcParams {
  /** The gas the caller is willing to pay for the transaction. */
  gas?: string;
  /** The deposit joined to the call. */
  amount?: string;
  /** The contract arguments. */
  args: any;
}

export interface ViewRpcParams {
  [name: string]: any;
}

export interface RpcCallsFt {
  ft_balance_of(params: ViewRpcParams): Promise<string>;
}

export interface RpcCallsStakingPool {
  get_account_staked_balance(params: ViewRpcParams): Promise<string>;
  get_account_total_balance(params: ViewRpcParams): Promise<string>;

  deposit(params: CallRpcParams): Promise<string>;
  deposit_and_stake_direct_stake(params: CallRpcParams): Promise<string>;
  deposit_and_stake(params: CallRpcParams): Promise<string>;
  stake(params: CallRpcParams): Promise<string>;
  withdraw(params: CallRpcParams): Promise<string>;
  withdraw_all(params: CallRpcParams): Promise<string>;
  unstake(params: CallRpcParams): Promise<string>;
  unstake_all(params: CallRpcParams): Promise<string>;
}

export interface RpcCallsOperator {
  get_validators(params: ViewRpcParams): Promise<ValidatorInfo[]>;
  get_number_of_accounts(params: ViewRpcParams): Promise<number>;
  get_accounts(params: ViewRpcParams): Promise<NearxAccount[]>;
  get_snapshot_users(params: ViewRpcParams): Promise<SnapshotUser[]>;

  'new'(params: CallRpcParams): Promise<ValidatorInfo[]>;
  upgrade(code: any, gas: any): Promise<string>;
  epoch_stake(params: CallRpcParams): Promise<string>;
  epoch_autocompound_rewards(params: CallRpcParams): Promise<string>;
  epoch_unstake(params: CallRpcParams): Promise<string>;
  epoch_withdraw(params: CallRpcParams): Promise<string>;
  sync_balance_from_validator(params: CallRpcParams): Promise<string>;
}

export interface RpcCallsUtils {
  get_current_epoch(params: ViewRpcParams): Promise<Epoch>;
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
        // Fungible Token:
        nameof<RpcCallsFt>('ft_balance_of'),
        // Staking Pool:
        nameof<RpcCallsStakingPool>('get_account_staked_balance'),
        nameof<RpcCallsStakingPool>('get_account_total_balance'),
        // Operator:
        nameof<RpcCallsOperator>('get_validators'),
        nameof<RpcCallsOperator>('get_number_of_accounts'),
        nameof<RpcCallsOperator>('get_snapshot_users'),
        nameof<RpcCallsOperator>('get_accounts'),
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
