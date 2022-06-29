import * as nearjs from 'near-api-js';
export * as nearjs from 'near-api-js';

export type Balance = bigint;
export type Epoch = bigint;
export type AccountId = string;

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

  /**
   * Returns a list of the validators.
   */
  validators(): Promise<ValidatorInfo[]>;

  /**
   * Returns the current epoch.
   */
  currentEpoch(): Promise<Epoch>;

  // User-facing methods:

  /**
   * Stake tokens inside the pool.
   */
  stake(amount: string): Promise<string>;

  /**
   * Unstake tokens from the pool.
   */
  unstake(amount: string): Promise<string>;
  /**
   * Unstake tokens from the pool.
   */
  unstakeAll(): Promise<string>;

  /**
   * Withdraw unstaked tokens from the pool.
   */
  withdraw(amount: string): Promise<string>;
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
   * Epoch autocompound rewards.
   */
  epochAutocompoundRewards(): Promise<any[]>;

  /**
   * Epoch unstake.
   */
  epochUnstake(): Promise<void>;

  /**
   * Epoch withdraw.
   * Returns a list of errors.
   */
  epochWithdraw(): Promise<any[]>;

  /**
   * Syncronize balance from validator.
   */
  syncBalances(): Promise<any[]>;
}

export interface NearxPoolClient extends NearxStakingPool {
  near: nearjs.Near;
  config: nearjs.ConnectConfig;
  contract: nearjs.Contract;
}
export { NearxPoolClient } from './nearx-pool-client';

// DTOs:

export interface ValidatorInfo {
  account_id: AccountId;
  staked: Balance;
  unstaked: Balance;
  last_asked_rewards_epoch_height: Epoch;
  last_unstake_start_epoch: Epoch;
  paused: boolean;
}
