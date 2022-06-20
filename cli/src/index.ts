import * as nearx from 'nearx-js';

const commands: { [name: string]: () => Promise<void> } = {
  stake: stake,
  unstake: unstake,
  withdraw: withdraw,
};

async function run(name: string) {
  if (name in commands) {
    await commands[name]();
  } else {
    if (name != null) {
      console.error('Undefined command:', name);
    }
    error();
  }
}

function error(message?: string): never {
  console.error(message ?? help);
  process.exit(1);
}

const help: string = `./nearx COMMAND
    COMMAND: ${Object.keys(commands).join(' | ')}`;

async function stake() {
  const client = await nearx.NearxPoolClient.new('testnet');

  await client.epochStake();
}

async function unstake() {
  console.log('unstake');
}

async function withdraw() {
  console.log('withdraw');
}

run(process.argv[2]).then(() => console.log('Command successfully executed'));
