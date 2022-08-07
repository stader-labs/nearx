import { NearxPoolClient } from './nearx-pool-client';

const upgrade = async (
  fileName: string,
  environment: 'testnet' | 'mainnet',
  contract: string,
  accountId: string,
) => {
  const nearxPoolClient = await NearxPoolClient.new(environment, contract, accountId);
  await nearxPoolClient.contractUpgrade(fileName);
};
