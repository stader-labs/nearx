import * as nearjs from 'near-api-js';
import { Epoch, ValidatorInfo } from '.';
import { nameof } from './utils';

export type NearxContract = nearjs.Contract &
  RpcCallsStakingPool &
  RpcCallsOperator &
  RpcCallsUtils;

export interface RpcCallsStakingPool {
  get_account_staked_balance(args: any): Promise<string>;
  get_account_unstaked_balance(args: any): Promise<string>;
  get_account_total_balance(args: any): Promise<string>;
  deposit(args: any, gas: undefined, deposit: string): Promise<string>;
  deposit_and_stake_direct_stake(args: any, gas: undefined, deposit: string): Promise<string>;
  deposit_and_stake(args: any, gas: undefined, deposit: string): Promise<string>;
  stake(args: any): Promise<string>;
  withdraw(args: any): Promise<string>;
  withdraw_all(args: any): Promise<string>;
  unstake(args: any): Promise<string>;
  unstake_all(args: any): Promise<string>;
  upgrade(code: any, gas: any): Promise<string>;
}

export interface RpcCallsOperator {
  get_validators(args: any): Promise<ValidatorInfo[]>;
  epoch_stake(args: any): Promise<string>;
  epoch_autocompound_rewards(args: any): Promise<string>;
  epoch_unstake(args: any): Promise<string>;
  epoch_withdraw(args: any): Promise<string>;
  sync_balance_from_validator(args: any): Promise<string>;
  upgrade(code: any, gas: any): Promise<any>;
}

export interface RpcCallsUtils {
  get_current_epoch(args: any): Promise<Epoch>;
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
        nameof<RpcCallsStakingPool>('get_account_unstaked_balance'),
        nameof<RpcCallsStakingPool>('get_account_total_balance'),
        // Operator:
        nameof<RpcCallsOperator>('get_validators'),
        // Utils:
        nameof<RpcCallsUtils>('get_current_epoch'),
      ],
      changeMethods: [
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
