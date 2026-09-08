export type MarketVenue = 'binance' | 'kucoin';

// Exchange market used for display-only metadata (24h change/high/low and the
// chart fallback when the oracle history is too short). Never feeds the oracle.
export interface MarketRef {
  venue: MarketVenue;
  symbol: string;
}

export interface FaucetConfig {
  faucetId: string;
  pair: string;
  name: string;
  marketCap: number;
  // Number of decimals the on-chain median is scaled by. Derived from
  // pragma-sdk's Pair.decimals() = min(base.decimals, quote.decimals).
  // USD has 8 decimals in pragma-sdk; base currency dictates min for
  // assets with fewer (e.g. USDT=6, XAUT=6).
  decimals: number;
  market: MarketRef;
}

const binance = (symbol: string): MarketRef => ({ venue: 'binance', symbol });
const kucoin = (symbol: string): MarketRef => ({ venue: 'kucoin', symbol });

// Must mirror STARKNET_PAIR_TO_MIDEN_FAUCET in pragma-sdk/pragma_sdk/miden/client.py.
// The Pragma price-pusher only forwards these pairs to Miden; adding a row
// here without a matching pusher mapping would just display zeros.
// USDT/USD has no direct counterpart on Binance: USDCUSDT is the closest 1:1
// reference. XMR is delisted from Binance (its API still serves a phantom
// XMRUSDT ~4x below the market), so it comes from KuCoin.
export const FAUCET_CONFIGS: FaucetConfig[] = [
  { faucetId: '1:0', pair: 'BTC/USD', name: 'Bitcoin', marketCap: 1_280_000_000_000, decimals: 8, market: binance('BTCUSDT') },
  { faucetId: '2:0', pair: 'ETH/USD', name: 'Ethereum', marketCap: 390_000_000_000, decimals: 8, market: binance('ETHUSDT') },
  { faucetId: '3:0', pair: 'WBTC/USD', name: 'Wrapped Bitcoin', marketCap: 13_500_000_000, decimals: 8, market: binance('WBTCUSDT') },
  { faucetId: '4:0', pair: 'USDT/USD', name: 'Tether', marketCap: 145_000_000_000, decimals: 6, market: binance('USDCUSDT') },
  { faucetId: '5:0', pair: 'DAI/USD', name: 'Dai', marketCap: 5_400_000_000, decimals: 8, market: binance('DAIUSDT') },
  { faucetId: '6:0', pair: 'ZEC/USD', name: 'Zcash', marketCap: 19_400_000_000, decimals: 8, market: binance('ZECUSDT') },
  { faucetId: '7:0', pair: 'XMR/USD', name: 'Monero', marketCap: 9_800_000_000, decimals: 8, market: kucoin('XMR-USDT') },
  { faucetId: '8:0', pair: 'DASH/USD', name: 'Dash', marketCap: 800_000_000, decimals: 8, market: binance('DASHUSDT') },
  { faucetId: '9:0', pair: 'XAUT/USD', name: 'Tether Gold', marketCap: 2_700_000_000, decimals: 6, market: binance('XAUTUSDT') },
  { faucetId: '10:0', pair: 'PAXG/USD', name: 'PAX Gold', marketCap: 1_900_000_000, decimals: 8, market: binance('PAXGUSDT') },
  { faucetId: '11:0', pair: 'LINK/USD', name: 'Chainlink', marketCap: 9_500_000_000, decimals: 8, market: binance('LINKUSDT') },
  { faucetId: '12:0', pair: 'UNI/USD', name: 'Uniswap', marketCap: 4_450_000_000, decimals: 8, market: binance('UNIUSDT') },
  { faucetId: '13:0', pair: 'AAVE/USD', name: 'Aave', marketCap: 2_000_000_000, decimals: 8, market: binance('AAVEUSDT') },
  { faucetId: '14:0', pair: 'MORPHO/USD', name: 'Morpho', marketCap: 1_650_000_000, decimals: 8, market: binance('MORPHOUSDT') },
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

export const PAIR_TO_MARKET = new Map(
  FAUCET_CONFIGS.map(config => [config.pair, config.market])
);
