import { exec } from 'child_process';
import { promisify } from 'util';
import { getCached, setCache } from '@/lib/cache';
import { fetchMultiple24hStats } from '@/lib/binance-api';
import type { Asset } from '@/types/asset';

const execAsync = promisify(exec);

const ORACLE_WORKSPACE = process.env.ORACLE_WORKSPACE_PATH || '';
const CLI_PATH = process.env.CLI_PATH || '';
const CACHE_TTL = 10000;
const MAX_RETRIES = 3;

const PAIRS = ['BTC/USD', 'ETH/USD', 'SOL/USD'];

const MARKET_CAPS: Record<string, number> = {
  'BTC/USD': 1_280_000_000_000,
  'ETH/USD': 390_000_000_000,
  'SOL/USD': 78_000_000_000,
};

const PAIR_NAMES: Record<string, string> = {
  'BTC/USD': 'Bitcoin',
  'ETH/USD': 'Ethereum',
  'SOL/USD': 'Solana',
};

interface MedianResult {
  pair: string;
  median: number;
}

async function fetchAllMediansWithRetry(pairs: string[]): Promise<Map<string, number>> {
  for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
    try {
      const { stdout } = await execAsync(
        `cd ${ORACLE_WORKSPACE} && ${CLI_PATH}/pm-oracle-cli median-batch ${pairs.join(' ')} --network testnet --json`,
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
      const binanceData = binanceStats.get(pair);
      
      const price = midenPrices.get(pair) || 0;
      const symbol = pair;
      const name = PAIR_NAMES[pair] || pair.split('/')[0];
      const marketCap = MARKET_CAPS[pair] || 0;
      
      return {
        symbol,
        name,
        price,
        change24h: binanceData?.change24h || 0,
        high24h: binanceData?.high24h || price,
        low24h: binanceData?.low24h || price,
        marketCap,
      };
    });
    
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
