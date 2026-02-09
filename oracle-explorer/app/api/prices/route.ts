import { exec } from 'child_process';
import { promisify } from 'util';
import { getCached, setCache } from '@/lib/cache';
import { fetchMultiple24hStats } from '@/lib/binance-api';
import { insertPriceHistory } from '@/lib/db';
import type { Asset } from '@/types/asset';

const execAsync = promisify(exec);

const NETWORK = process.env.NETWORK || 'testnet';
const ORACLE_WORKSPACE = process.env.ORACLE_WORKSPACE_PATH || '';
const CLI_PATH = process.env.CLI_PATH || '';
const CACHE_TTL = 4000;
const MAX_RETRIES = 3;

const PAIRS = ['BTC/USD', 'ETH/USD', 'SOL/USD', 'BNB/USD', 'XRP/USD', 'HYPE/USD', 'POL/USD'];

const MARKET_CAPS: Record<string, number> = {
  'BTC/USD': 1_280_000_000_000,
  'ETH/USD': 390_000_000_000,
  'SOL/USD': 78_000_000_000,
  'BNB/USD': 85_000_000_000,
  'XRP/USD': 140_000_000_000,
  'HYPE/USD': 3_500_000_000,
  'POL/USD': 7_500_000_000,
};

const PAIR_NAMES: Record<string, string> = {
  'BTC/USD': 'Bitcoin',
  'ETH/USD': 'Ethereum',
  'SOL/USD': 'Solana',
  'BNB/USD': 'BNB',
  'XRP/USD': 'XRP',
  'HYPE/USD': 'Hyperliquid',
  'POL/USD': 'Polygon',
};

interface MedianResult {
  pair: string;
  median: number;
}

async function fetchAllMediansWithRetry(pairs: string[]): Promise<Map<string, number>> {
  for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
    try {
      const { stdout } = await execAsync(
        `cd ${ORACLE_WORKSPACE} && ${CLI_PATH}/pm-oracle-cli median-batch ${pairs.join(' ')} --network ${NETWORK} --json`,
        { timeout: 30000 }
      );
      
      const lines = stdout.trim().split('\n');
      const jsonLine = lines.find(line => line.startsWith('['));
      
      if (jsonLine) {
        const results: MedianResult[] = JSON.parse(jsonLine);
        const priceMap = new Map<string, number>();
        
        results.forEach(({ pair, median }) => {
          priceMap.set(pair, median / 1_000_000);
        });
        
        return priceMap;
      }
    } catch (error) {
      if (attempt === MAX_RETRIES - 1) {
        throw error;
      }
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
  }
  
  throw new Error('Failed to fetch medians after retries');
}

function storeLivePrices(priceMap: Map<string, number>): void {
  try {
    const timestamp = Math.floor(Date.now() / 1000);
    
    const rows = Array.from(priceMap.entries()).map(([pair, price]) => ({
      pair,
      price,
      decimals: 6,
      timestamp,
    }));
    
    insertPriceHistory(rows);
  } catch (error) {
    console.error('Failed to store live prices:', error);
  }
}

export async function GET() {
  try {
    const cacheKey = 'prices:all';
    
    const cached = getCached<Asset[]>(cacheKey);
    if (cached) {
      return Response.json(cached);
    }
    
    const [midenPrices, binanceStats] = await Promise.all([
      fetchAllMediansWithRetry(PAIRS),
      fetchMultiple24hStats(PAIRS),
    ]);

    const assets: Asset[] = PAIRS.map((pair) => {
      const price = midenPrices.get(pair) || 0;
      const symbol = pair;
      const name = PAIR_NAMES[pair] || pair.split('/')[0];
      const marketCap = MARKET_CAPS[pair] || 0;
      
      const binanceData = binanceStats.get(pair);
      const stats24h = binanceData || { change24h: 0, high24h: price, low24h: price };
      
      return {
        symbol,
        name,
        price,
        change24h: stats24h.change24h,
        high24h: stats24h.high24h,
        low24h: stats24h.low24h,
        marketCap,
      };
    });
    
    storeLivePrices(midenPrices);
    
    setCache(cacheKey, assets, CACHE_TTL);
    
    return Response.json(assets);
  } catch (error) {
    console.error('Prices API error:', error);
    return Response.json(
      { error: 'Failed to fetch prices', details: String(error) },
      { status: 500 }
    );
  }
}
