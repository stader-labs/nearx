import * as nearjs from 'near-api-js';
import { nameof } from './utils';

export type NearxContract = nearjs.Contract & RpcCallsStakingPool & RpcCallsOperator;

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
}

export interface RpcCallsOperator {
  epoch_stake(args: any): Promise<string>;
  epoch_unstake(args: any): Promise<string>;
  epoch_withdraw(args: any): Promise<string>;
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
        nameof<RpcCallsStakingPool>('get_account_staked_balance'),
        nameof<RpcCallsStakingPool>('get_account_unstaked_balance'),
        nameof<RpcCallsStakingPool>('get_account_total_balance'),
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
        nameof<RpcCallsOperator>('epoch_unstake'),
        nameof<RpcCallsOperator>('epoch_withdraw'),
      ],
      //sender: account, // account object to initialize and sign transactions.
    },
  ) as NearxContract;
}
