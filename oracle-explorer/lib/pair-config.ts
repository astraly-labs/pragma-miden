export type MarketVenue = 'binance' | 'kucoin';

// Exchange market used for display-only metadata (24h change/high/low and the
// chart fallback when the oracle history is too short). Never feeds the oracle.
export interface MarketRef {
  venue: MarketVenue;
  symbol: string;
}

export interface PairConfig {
  pair: string;
  // On-chain key of the pair: the Poseidon2 hash of its name (see docs/oracle-v1-consumer.md; the
  // values are frozen in crates/accounts/tests/test_pair_key.rs). Display only: the CLI derives it.
  key: string;
  name: string;
  marketCap: number;
  // Decimals the publishers use for the pair (min(base, quote) in pragma-sdk). The oracle answers
  // with them, so the API reads them from the answer; this value is for display only.
  decimals: number;
  market: MarketRef;
}

const binance = (symbol: string): MarketRef => ({ venue: 'binance', symbol });
const kucoin = (symbol: string): MarketRef => ({ venue: 'kucoin', symbol });

// Must mirror MIDEN_PAIRS in pragma-sdk/pragma_sdk/miden/client.py.
// The Pragma price-pusher only forwards these pairs to Miden; adding a row
// here without a matching pusher mapping would just display zeros.
// USDT/USD and USDC/USD have no direct counterpart on Binance: USDCUSDT is the
// closest 1:1 reference (also the stand-in feed for USDCx). XMR is delisted from Binance (its API still serves a phantom
// XMRUSDT ~4x below the market), so it comes from KuCoin.
export const PAIR_CONFIGS: PairConfig[] = [
  { pair: 'BTC/USD', key: '0xd189b4e9c38c367ba926ddb0044b781ca8fff18dcc10a0c75ff5600a1eb4d0eb', name: 'Bitcoin', marketCap: 1_280_000_000_000, decimals: 8, market: binance('BTCUSDT') },
  { pair: 'ETH/USD', key: '0x9a50f4137c634e0b384a87bea3a186e578e21274af6ae752230ead4631233e1d', name: 'Ethereum', marketCap: 390_000_000_000, decimals: 8, market: binance('ETHUSDT') },
  { pair: 'WBTC/USD', key: '0x4c972cdae7848e436e54fc4a19fa00ad5eded1b9c959744e16485a1d673217cd', name: 'Wrapped Bitcoin', marketCap: 13_500_000_000, decimals: 8, market: binance('WBTCUSDT') },
  { pair: 'USDT/USD', key: '0x6cbc5578f2e294131c429b5be03ef87692da0d4300816ae7238058817c6a8085', name: 'Tether', marketCap: 145_000_000_000, decimals: 6, market: binance('USDCUSDT') },
  { pair: 'DAI/USD', key: '0x59660b43348776410276dc5bc2934ca549660b0faf88299acc7a003283f90e6e', name: 'Dai', marketCap: 5_400_000_000, decimals: 8, market: binance('DAIUSDT') },
  { pair: 'ZEC/USD', key: '0x39179ab0864e74f9f5160fa3bcf182ed2ee70a81ff40784b561d197fe8e25681', name: 'Zcash', marketCap: 19_400_000_000, decimals: 8, market: binance('ZECUSDT') },
  { pair: 'XMR/USD', key: '0xc7abe26df1eab19ff1b48f8ecbf31efda088362551f2c026e0f23add86dc3cbe', name: 'Monero', marketCap: 9_800_000_000, decimals: 8, market: kucoin('XMR-USDT') },
  { pair: 'DASH/USD', key: '0x2184974137b3e2018306c63381590e30b2cbffc503a7e7a4003a55fb1c9fede2', name: 'Dash', marketCap: 800_000_000, decimals: 8, market: binance('DASHUSDT') },
  { pair: 'XAUT/USD', key: '0xb6246558eb55426ad719c05c058b66f87da325120789876aa5e3af156f90896c', name: 'Tether Gold', marketCap: 2_700_000_000, decimals: 6, market: binance('XAUTUSDT') },
  { pair: 'PAXG/USD', key: '0x286672fe34caa39c41a593157ecdcdb88543b12e462ad8dea33f685039acaba2', name: 'PAX Gold', marketCap: 1_900_000_000, decimals: 8, market: binance('PAXGUSDT') },
  { pair: 'LINK/USD', key: '0x47966fdc920a12359900a930cd5ec3e5c583e542a2ca4b5061013d401c55fc80', name: 'Chainlink', marketCap: 9_500_000_000, decimals: 8, market: binance('LINKUSDT') },
  { pair: 'UNI/USD', key: '0x28b5da79b6aab6289ab569fc3b679226b0d113fb1d9f0df62410a93241b3c29c', name: 'Uniswap', marketCap: 4_450_000_000, decimals: 8, market: binance('UNIUSDT') },
  { pair: 'AAVE/USD', key: '0x8f738295d669c20ccd2fd738b2284def832da2ded53b9afc22e8746f1c84c7e6', name: 'Aave', marketCap: 2_000_000_000, decimals: 8, market: binance('AAVEUSDT') },
  { pair: 'MORPHO/USD', key: '0xfd585c6cd7a15d199c4914a57e5533c8327e31eb3515099002d1add2647bb23b', name: 'Morpho', marketCap: 1_650_000_000, decimals: 8, market: binance('MORPHOUSDT') },
  { pair: 'USDC/USD', key: '0x1dfbb6c4acb913805c4df6731bfe7bfb8b6acd999bb768feb24979170807ee51', name: 'USD Coin', marketCap: 74_000_000_000, decimals: 6, market: binance('USDCUSDT') },
];

export const PAIR_TO_MARKET = new Map(
  PAIR_CONFIGS.map(config => [config.pair, config.market])
);
