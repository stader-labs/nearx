import * as nearjs from 'near-api-js';
export * as nearjs from 'near-api-js';

export type Balance = string;

export type Network = 'testnet' | 'mainnet';

export interface NearxStakingPool {
  // View methods:

  /**
   * Returns the user's number of tokens unstaked inside the pool.
   */
  unstakedBalance(): Promise<Balance>;

  /**
   * Returns the user's number of tokens staked inside the pool.
   */
  stakedBalance(): Promise<Balance>;

  /**
   * Returns the user's total number of tokens inside the pool
   * (both staked and unstaked).
   */
  totalBalance(): Promise<Balance>;

  // User-facing methods:

  /**
   * Stake tokens inside the pool.
   */
  stake(amount: Balance): Promise<string>;

  /**
   * Unstake tokens from the pool.
   */
  unstake(amount: Balance): Promise<string>;
  /**
   * Unstake tokens from the pool.
   */
  unstakeAll(): Promise<string>;

  /**
   * Withdraw unstaked tokens from the pool.
   */
  withdraw(amount: Balance): Promise<string>;
  /**
   * Withdraw unstaked tokens from the pool.
   */
  withdrawAll(): Promise<string>;

  // Operator methods:

  /**
   * Epoch stake.
   */
  epochStake(): Promise<string>;

  /**
   * Epoch stake.
   */
  epochUnstake(): Promise<string>;

  /**
   * Epoch stake.
   */
  epochWithdraw(): Promise<string>;
}

export interface NearxPoolClient extends NearxStakingPool {
  near: nearjs.Near;
  config: nearjs.ConnectConfig;
  contract: nearjs.Contract;
}
export { NearxPoolClient } from './nearx-pool-client';
