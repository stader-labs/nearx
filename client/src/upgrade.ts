import { NearxPoolClient } from './nearx-pool-client';

const upgrade = async (fileName: string) => {
  const nearxPoolClient = await NearxPoolClient.new(
    'testnet',
    'nearx.staderlabs.testnet',
    'staderlabs.testnet',
  );
  await nearxPoolClient.contractUpgrade(fileName);
};
