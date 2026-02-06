/**
 * Binance API helper to fetch 24h ticker statistics
 * Used for metadata (change24h, high24h, low24h) in the Oracle Explorer
 */

interface Binance24hTicker {
  symbol: string;
  priceChange: string;
  priceChangePercent: string;
  lastPrice: string;
  highPrice: string;
  lowPrice: string;
}

const SYMBOL_MAP: Record<string, string> = {
  'BTC/USD': 'BTCUSDT',
  'ETH/USD': 'ETHUSDT',
  'SOL/USD': 'SOLUSDT',
  'BNB/USD': 'BNBUSDT',
  'XRP/USD': 'XRPUSDT',
  'POL/USD': 'POLUSDT',
};

/**
 * Fetch 24h statistics from Binance API
 * @param pair - Trading pair in format "BTC/USD"
 * @returns 24h stats including change%, high, low
 */
export async function fetch24hStats(pair: string): Promise<{
  change24h: number;
  high24h: number;
  low24h: number;
} | null> {
  try {
    if (pair === 'HYPE/USD') {
      const response = await fetch(
        `https://api.bybit.com/v5/market/tickers?category=spot&symbol=HYPEUSDT`,
        {
          next: { revalidate: 10 },
        }
      );

      if (!response.ok) {
        console.error(`Bybit API error: ${response.status}`);
        return null;
      }

      const data = await response.json();
      const ticker = data.result?.list?.[0];
      
      if (!ticker) {
        return null;
      }

      return {
        change24h: parseFloat(ticker.price24hPcnt) * 100,
        high24h: parseFloat(ticker.highPrice24h),
        low24h: parseFloat(ticker.lowPrice24h),
      };
    }

    const binanceSymbol = SYMBOL_MAP[pair];
    if (!binanceSymbol) {
      console.warn(`No Binance mapping for pair: ${pair}`);
      return null;
    }

    const response = await fetch(
      `https://api.binance.com/api/v3/ticker/24hr?symbol=${binanceSymbol}`,
      {
        next: { revalidate: 10 },
      }
    );

    if (!response.ok) {
      console.error(`Binance API error: ${response.status}`);
      return null;
    }

    const data: Binance24hTicker = await response.json();

    return {
      change24h: parseFloat(data.priceChangePercent),
      high24h: parseFloat(data.highPrice),
      low24h: parseFloat(data.lowPrice),
    };
  } catch (error) {
    console.error(`Error fetching 24h stats for ${pair}:`, error);
    return null;
  }
}

/**
 * Fetch 24h stats for multiple pairs in parallel
 * @param pairs - Array of trading pairs
 * @returns Map of pair -> stats
 */
export async function fetchMultiple24hStats(
  pairs: string[]
): Promise<Map<string, { change24h: number; high24h: number; low24h: number }>> {
  const results = await Promise.all(
    pairs.map(async (pair) => {
      const stats = await fetch24hStats(pair);
      return { pair, stats };
    })
  );

  const statsMap = new Map();
  results.forEach(({ pair, stats }) => {
    if (stats) {
      statsMap.set(pair, stats);
    }
  });

  return statsMap;
}
