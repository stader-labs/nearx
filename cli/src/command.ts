import * as nearx from 'nearx-js';

function near(amount: number): string {
  return Math.round(amount).toString() + '0'.repeat(24);
}

export async function displaySnapshot(client: nearx.NearxPoolClient): Promise<void> {
  console.log(await client.userAccounts());
}

export async function displayBalance(client: nearx.NearxPoolClient): Promise<void> {
  console.log({
    staked: (await client.stakedBalance()).toString(),
    total: (await client.totalBalance()).toString(),
  });
}

export async function displayValidators(client: nearx.NearxPoolClient): Promise<void> {
  for (const validator of await client.validators()) {
    console.log(validator);
  }
}

// Used for tests only (at least for now):
export async function runInit(client: nearx.NearxPoolClient, accountId: string): Promise<void> {
  logCommand('init');

  await client.contract.new({
    args: {
      owner_account_id: accountId,
      operator_account_id: accountId,
      treasury_account_id: accountId,
    },
  });
}

export async function syncBalances(client: nearx.NearxPoolClient): Promise<void> {
  logCommand('sync balances');
  await client.syncBalances();
}

export async function epochAutocompoundRewards(client: nearx.NearxPoolClient): Promise<void> {
  logCommand('epoch autocompound');
  await client.epochAutocompoundRewards();
}

export async function stake(client: nearx.NearxPoolClient): Promise<void> {
  logCommand('epoch stake');
  await client.epochStake();
}

export async function unstake(client: nearx.NearxPoolClient): Promise<void> {
  logCommand('epoch unstake');
  await client.epochUnstake();
}

export async function withdraw(client: nearx.NearxPoolClient): Promise<void> {
  logCommand('epoch withdraw');
  await client.epochWithdraw();
}

function logCommand(...name: any[]) {
  console.debug('\n> Running', ...name);
}

export async function runWholeEpoch(client: nearx.NearxPoolClient): Promise<void> {
  //await syncBalances(client);
  await epochAutocompoundRewards(client);
  await stake(client);
  await unstake(client);
  await withdraw(client);
}

// User

export async function userDeposit(client: nearx.NearxPoolClient): Promise<void> {
  const amount = near(2);
  logCommand('user deposit:', amount);
  await client.stake(amount);
}
