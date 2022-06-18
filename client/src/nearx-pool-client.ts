import * as nearjs from 'near-api-js';
import { Balance, NearxPoolClient as Iface } from '.';
import { createContract } from './contract';

type NearxPoolClient = Iface;
export const NearxPoolClient = {
  async new(networkId: 'testnet' | 'mainnet'): Promise<NearxPoolClient> {
    const config = configFromNetwork(networkId);
    // Connect to NEAR:
    const near = await nearjs.connect(config);
    // Create wallet connection:
    const wallet = new nearjs.WalletConnection(near, null);

    const contract = createContract(wallet);

    return {
      near,
      config,
      wallet,
      contract,

      // View methods:
      async stakedBalance(): Promise<Balance> {
        return contract.get_account_staked_balance({
          account_id: wallet.getAccountId(),
        });
      },
      async unstakedBalance(): Promise<Balance> {
        return contract.get_account_unstaked_balance({
          account_id: wallet.getAccountId(),
        });
      },
      async totalBalance(): Promise<Balance> {
        return contract.get_account_total_balance({
          account_id: wallet.getAccountId(),
        });
      },

      // User-facing methods:
      async stake(amount: string): Promise<string> {
        throw new Error('Not implemented');
      },

      async unstake(amount: string): Promise<string> {
        throw new Error('Not implemented');
      },

      async unstakeAll(): Promise<string> {
        throw new Error('Not implemented');
      },

      async withdraw(amount: string): Promise<string> {
        throw new Error('Not implemented');
      },

      async withdrawAll(): Promise<string> {
        throw new Error('Not implemented');
      },

      // Operator methods:
      async epochStake(): Promise<string> {
        let result = 0;

        while (await contract.epoch_stake({})) {
          result += 1;
        }

        return `Successfully staked ${result} times`;
      },

      async epochUnstake(): Promise<string> {
        let result = 0;

        while (await contract.epoch_stake({})) {
          result += 1;
        }

        return `Successfully staked ${result} times`;
      },

      async epochWithdraw(): Promise<string> {
        let result = 0;

        while (await contract.epoch_stake({})) {
          result += 1;
        }

        return `Successfully staked ${result} times`;
      },
    };
  },
};

function configFromNetwork(networkId: string): nearjs.ConnectConfig {
  const config = {
    keyStore: new nearjs.keyStores.BrowserLocalStorageKeyStore(),
    networkId,
    headers: {},
  };

  switch (networkId) {
    case 'testnet':
      return {
        ...config,
        helperUrl: 'https://helper.testnet.near.org',
        nodeUrl: 'https://rpc.testnet.near.org',
        walletUrl: 'https://wallet.testnet.near.org',
      };
    case 'mainnet':
      return {
        ...config,
        helperUrl: 'https://helper.near.org',
        nodeUrl: 'https://rpc.near.org',
        walletUrl: 'https://wallet.near.org',
      };
    default:
      throw new Error('Invalid network: ' + networkId);
  }
}
