export interface FaucetConfig {
  faucetId: string;
  pair: string;
  name: string;
  marketCap: number;
  // Number of decimals the on-chain median is scaled by. Derived from
  // pragma-sdk's Pair.decimals() = min(base.decimals, quote.decimals).
  // USD has 8 decimals in pragma-sdk; base currency dictates min for
  // assets with fewer (e.g. USDT=6).
  decimals: number;
}

// Must mirror STARKNET_PAIR_TO_MIDEN_FAUCET in pragma-sdk/pragma_sdk/miden/client.py.
// The Pragma price-pusher only forwards these 5 pairs to Miden; adding a row
// here without a matching pusher mapping would just display zeros.
export const FAUCET_CONFIGS: FaucetConfig[] = [
  { faucetId: '1:0', pair: 'BTC/USD', name: 'Bitcoin', marketCap: 1_280_000_000_000, decimals: 8 },
  { faucetId: '2:0', pair: 'ETH/USD', name: 'Ethereum', marketCap: 390_000_000_000, decimals: 8 },
  { faucetId: '3:0', pair: 'WBTC/USD', name: 'Wrapped Bitcoin', marketCap: 13_500_000_000, decimals: 8 },
  { faucetId: '4:0', pair: 'USDT/USD', name: 'Tether', marketCap: 145_000_000_000, decimals: 6 },
  { faucetId: '5:0', pair: 'DAI/USD', name: 'Dai', marketCap: 5_400_000_000, decimals: 8 },
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

export const FAUCET_ID_TO_DECIMALS = new Map(
  FAUCET_CONFIGS.map(config => [config.faucetId, config.decimals])
);
