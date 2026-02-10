export interface FaucetConfig {
  faucetId: string;
  pair: string;
  name: string;
  marketCap: number;
}

export const FAUCET_CONFIGS: FaucetConfig[] = [
  { faucetId: '1:0', pair: 'BTC/USD', name: 'Bitcoin', marketCap: 1_280_000_000_000 },
  { faucetId: '2:0', pair: 'ETH/USD', name: 'Ethereum', marketCap: 390_000_000_000 },
  { faucetId: '3:0', pair: 'SOL/USD', name: 'Solana', marketCap: 78_000_000_000 },
  { faucetId: '4:0', pair: 'BNB/USD', name: 'BNB', marketCap: 85_000_000_000 },
  { faucetId: '5:0', pair: 'XRP/USD', name: 'XRP', marketCap: 140_000_000_000 },
  { faucetId: '6:0', pair: 'HYPE/USD', name: 'Hyperliquid', marketCap: 3_500_000_000 },
  { faucetId: '7:0', pair: 'POL/USD', name: 'Polygon', marketCap: 7_500_000_000 },
];

export const FAUCET_ID_TO_PAIR = new Map(
  FAUCET_CONFIGS.map(config => [config.faucetId, config.pair])
);

export const PAIR_TO_FAUCET_ID = new Map(
  FAUCET_CONFIGS.map(config => [config.pair, config.faucetId])
);

export const FAUCET_ID_TO_NAME = new Map(
  FAUCET_CONFIGS.map(config => [config.faucetId, config.name])
);

export const FAUCET_ID_TO_MARKET_CAP = new Map(
  FAUCET_CONFIGS.map(config => [config.faucetId, config.marketCap])
);
