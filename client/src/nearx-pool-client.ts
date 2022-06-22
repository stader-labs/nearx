import * as nearjs from 'near-api-js';
import { Balance, NearxPoolClient as Iface, Network } from '.';
import { createContract, NearxContract } from './contract';
import * as os from 'os';
import { isBrowser } from './utils';

type NearxPoolClient = Iface;
export const NearxPoolClient = {
  async new(
    networkId: 'testnet' | 'mainnet',
    contractName: string,
    // Local account:
    accountId?: string,
  ): Promise<NearxPoolClient> {
    // Depending on being in the browser or not,
    // the config is set from a local keystore or the browser wallet:
    const config = configFromNetwork(networkId);
    // Connect to NEAR:
    const near = await nearjs.connect(config);

    let contract: NearxContract;

    if (isBrowser()) {
      console.debug('Client created from the browser');
      const wallet = new nearjs.WalletAccount(near, null);

      contract = createContract(wallet.account(), contractName);
      accountId = wallet.getAccountId();
    } else {
      console.debug('Client created from a CLI');
      if (accountId == null) {
        throw new Error('When used in a CLI, the accountId must be specified');
      }
      // Use the previously set keystore:
      const account = new nearjs.Account(near.connection, accountId);

      contract = createContract(account, contractName);
    }

    return {
      near,
      config,
      contract,

      // View methods:
      async stakedBalance(): Promise<Balance> {
        return contract.get_account_staked_balance({
          account_id: accountId,
        });
      },
      async unstakedBalance(): Promise<Balance> {
        return contract.get_account_unstaked_balance({
          account_id: accountId,
        });
      },
      async totalBalance(): Promise<Balance> {
        return contract.get_account_total_balance({
          account_id: accountId,
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

        while (await contract.epoch_stake({ args: {} })) {
          result += 1;
        }

        return `Successfully staked ${result} times`;
      },

      async epochUnstake(): Promise<string> {
        let result = 0;

        while (await contract.epoch_stake({ args: {} })) {
          result += 1;
        }

        return `Successfully staked ${result} times`;
      },

      async epochWithdraw(): Promise<string> {
        let result = 0;

        while (await contract.epoch_stake({ args: {} })) {
          result += 1;
        }

        return `Successfully staked ${result} times`;
      },
    };
  },
};

function localAccountPath(): string {
  return `${os.homedir()}/.near-credentials`;
}

function configFromNetwork(networkId: Network): nearjs.ConnectConfig {
  const keyStore = isBrowser()
    ? new nearjs.keyStores.BrowserLocalStorageKeyStore()
    : new nearjs.keyStores.UnencryptedFileSystemKeyStore(localAccountPath());
  const config = {
    keyStore,
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
