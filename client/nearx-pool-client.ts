import * as near from 'near-api-js';
import { Balance, NearxPoolClient } from '.';

export const NearxPoolClient_ = {
  async new(networkId: 'testnet' | 'mainnet'): Promise<NearxPoolClient> {
    const config = configFromNetwork(networkId);
    // Connect to NEAR:
    const connection = await near.connect(config);
    // Create wallet connection:
    // @ts-ignore
    const wallet = new near.WalletConnection(connection, null);

    //const contract = new near.Contract(
    //  account, // the account object that is connecting
    //  'example-contract.testnet',
    //  {
    //    // name of contract you're connecting to
    //    viewMethods: ['getMessages'], // view methods do not change state but usually return a value
    //    changeMethods: ['addMessage'], // change methods modify state
    //    sender: account, // account object to initialize and sign transactions.
    //  },
    //);

    return {
      connection,
      config,

      async unstakedBalance(wallet: near.WalletConnection): Promise<Balance> {
        return '';
      },
      async stakedBalance(wallet: near.WalletConnection): Promise<Balance> {
        return '';
      },
      async totalBalance(wallet: near.WalletConnection): Promise<Balance> {
        return '';
      },

      // Operations:
      async stake(wallet: near.WalletConnection, amount: string): Promise<void> {
        //
      },

      async unstake(wallet: near.WalletConnection, amount: string): Promise<void> {
        //
      },

      async unstakeAll(wallet: near.WalletConnection): Promise<void> {
        //
      },

      async withdraw(wallet: near.WalletConnection, amount: string): Promise<void> {
        //
      },

      async withdrawAll(wallet: near.WalletConnection): Promise<void> {
        //
      },

      // Private:
    };
  },
};

function configFromNetwork(networkId: string): near.ConnectConfig {
  const config = {
    keyStore: new near.keyStores.BrowserLocalStorageKeyStore(),
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
