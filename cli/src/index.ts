import * as nearx from 'nearx-js';

const commands: {
  [name: string]: (client: nearx.NearxPoolClient) => Promise<string>;
} = {
  stake: stake,
  unstake: unstake,
  withdraw: withdraw,
};

async function run(networkContract: string, accountId: string, commandName: string) {
  const [network_, contractName, ...rest] = networkContract.split(':');
  if (rest.length != 0) {
    error('Invalid network and contract name');
  }

  if (commandName in commands) {
    const network = typedNetwork(network_);
    accountId = canonicalAccountId(network, accountId);

    console.debug({ commandName, network, contractName, accountId });

    const client = await nearx.NearxPoolClient.new(network, contractName, accountId);

    await commands[commandName](client);
  } else {
    if (commandName != null) {
      console.error('Undefined command:', commandName);
    }
    error();
  }
}

function error(message?: string): never {
  console.error(message ?? help);
  process.exit(1);
}

const help: string = `Usage:

./nearx <network>:<contract name> <account ID> COMMAND
    COMMAND: ${Object.keys(commands).join(' | ')}`;

async function stake(client: nearx.NearxPoolClient): Promise<string> {
  return client.epochStake();
}

async function unstake(client: nearx.NearxPoolClient): Promise<string> {
  return client.epochUnstake();
}

async function withdraw(client: nearx.NearxPoolClient): Promise<string> {
  return client.epochWithdraw();
}

run(process.argv[2], process.argv[3], process.argv[4]).then(() =>
  console.log('Command successfully executed'),
);

function typedNetwork(s: string): nearx.Network {
  switch (s) {
    case 'testnet':
    case 'mainnet':
      return s;
    default:
      error(`Invalid network: ${s}`);
  }
}

function canonicalAccountId(networkId: nearx.Network, accountId: string): string {
  if (accountId.split('.')[1] != undefined) {
    return accountId;
  }

  switch (networkId) {
    case 'mainnet':
      return accountId + '.near';
    case 'testnet':
      return accountId + '.testnet';
    default:
      throw new Error('Invalid network: ' + networkId);
  }
}
