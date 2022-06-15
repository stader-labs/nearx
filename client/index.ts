import * as near from 'near-api-js';
export * as near from 'near-api-js';

export type Balance = string;

export interface NearxPoolClient {
  connection: near.Near;
  config: near.ConnectConfig;

  /**
   * Returns the user's number of tokens unstaked inside the pool.
   */
  unstakedBalance(wallet: near.WalletConnection): Promise<Balance>;
  /**
   * Returns the user's number of tokens staked inside the pool.
   */
  stakedBalance(wallet: near.WalletConnection): Promise<Balance>;
  /**
   * Returns the user's total number of tokens inside the pool
   * (both staked and unstaked).
   */
  totalBalance(wallet: near.WalletConnection): Promise<Balance>;

  /**
   * Stake tokens inside the pool.
   */
  stake(wallet: near.WalletConnection, amount: Balance): Promise<void>;

  /**
   * Unstake tokens from the pool.
   */
  unstake(wallet: near.WalletConnection, amount: Balance): Promise<void>;
  /**
   * Unstake tokens from the pool.
   */
  unstakeAll(wallet: near.WalletConnection): Promise<void>;

  /**
   * Withdraw unstaked tokens from the pool.
   */
  withdraw(wallet: near.WalletConnection, amount: Balance): Promise<void>;
  /**
   * Withdraw unstaked tokens from the pool.
   */
  withdrawAll(wallet: near.WalletConnection): Promise<void>;

  [privateMembers: string]: any;
}
export { NearxPoolClient_ as NearxPoolClient } from './nearx-pool-client';
