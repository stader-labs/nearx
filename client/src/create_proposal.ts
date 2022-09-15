import { Account, connect, keyStores } from 'near-api-js';

const createProposal = async (contract: string) => {
  const homedir = require('os').homedir();
  const CREDENTIALS_DIR = '.near-credentials';
  const credentialsPath = require('path').join(homedir, CREDENTIALS_DIR);
  const myKeyStore = new keyStores.UnencryptedFileSystemKeyStore(credentialsPath);

  const connectionConfig = {
    networkId: 'testnet',
    keyStore: myKeyStore, // first create a key store
    nodeUrl: 'https://rpc.testnet.near.org',
    walletUrl: 'https://wallet.testnet.near.org',
    helperUrl: 'https://helper.testnet.near.org',
    explorerUrl: 'https://explorer.testnet.near.org',
    headers: {},
  };
  const nearConnection = await connect(connectionConfig);
  const account = new Account(nearConnection.connection, 'staderlabs.testnet');

  //const acc = await nearxPoolClient.near.account('staderlabs.testnet');
  const addProposalArgs = {
    proposal: {
      description: 'Testing',
      kind: {
        FunctionCall: {
          receiver_id: 'v2-nearx.staderlabs.testnet',
          actions: [
            {
              method_name: 'deposit_and_stake',
              args: 'e30=',
              deposit: '1000000000000000000000000',
              gas: '150000000000000',
            },
          ],
        },
      },
    },
  };

  await account.functionCall({
    contractId: contract,
    methodName: 'add_proposal',
    args: addProposalArgs,
    attachedDeposit: '100000000000000000000000',
    gas: '150000000000000',
  });
};

createProposal('slackie-12345.sputnikv2.testnet').then(console.log);
