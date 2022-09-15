import { NearxPoolClient } from './nearx-pool-client';

const upgrade = async (
  fileName: string,
  environment: 'testnet' | 'mainnet',
  contract: string,
  accountId: string,
) => {
  const nearxPoolClient = await NearxPoolClient.new(environment, contract, accountId);
  // nearxPoolClient.near.
  await nearxPoolClient.contractUpgrade(fileName);
};

// upgrade(
//   '/Users/bharath12345/stader-work/near-liquid-token/res/near_x.wasm',
//   'mainnet',
//   'v2-nearx.stader-labs.near',
//   'stader-labs.near',
// ).then(console.log);
