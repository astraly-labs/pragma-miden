import { exec } from 'child_process';
import { promisify } from 'util';
import { getCached, setCache } from '@/lib/cache';
import { fetchMultiple24hStats } from '@/lib/market-data';
import { insertPriceHistory } from '@/lib/db';
import type { Asset } from '@/types/asset';
import { PAIR_CONFIGS } from '@/lib/pair-config';

const execAsync = promisify(exec);

const NETWORK = process.env.NETWORK || 'testnet';
const ORACLE_WORKSPACE = process.env.ORACLE_WORKSPACE_PATH || '';
const CLI_PATH = process.env.CLI_PATH || '';
const CACHE_TTL = 10000; // Increased from 4s to 10s to reduce spawn frequency
const MAX_RETRIES = 3;

// Lock to prevent multiple concurrent CLI spawns (cache stampede prevention)
const pendingRequests = new Map<string, Promise<Asset[]>>();

// What `pm-oracle-cli median-batch --json` answers for each pair. `num_publishers` is the number of
// publishers whose fresh entry is in the median: 0 means there is no price (nobody published, the
// entries are too old, or the oracle is paused), and `median` and `decimals` are then 0.
interface PairRead {
  pair: string;
  num_publishers: number;
  median: number;
  decimals: number;
  last_updated: number;
}

// Price and decimals of every pair that has a price.
interface Priced {
  price: number;
  decimals: number;
}

async function fetchAllMediansWithRetry(pairs: string[]): Promise<Map<string, Priced>> {
  for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
    try {
      const { stdout } = await execAsync(
        `cd ${ORACLE_WORKSPACE} && ${CLI_PATH}/pm-oracle-cli median-batch ${pairs.join(' ')} --network ${NETWORK} --json`,
        { timeout: 30000 }
      );

      const lines = stdout.trim().split('\n');
      const jsonLine = lines.find(line => line.startsWith('['));

      if (jsonLine) {
        const results: PairRead[] = JSON.parse(jsonLine);
        const priced = new Map<string, Priced>();

        results.forEach(({ pair, num_publishers, median, decimals }) => {
          if (num_publishers === 0) return;
          priced.set(pair, { price: median / Math.pow(10, decimals), decimals });
        });

        return priced;
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

function storeLivePrices(priced: Map<string, Priced>): void {
  try {
    const timestamp = Math.floor(Date.now() / 1000);

    const rows = Array.from(priced.entries()).map(([pair, { price, decimals }]) => ({
      pair,
      price,
      decimals,
      timestamp,
    }));

    insertPriceHistory(rows);
  } catch (error) {
    console.error('Failed to store live prices:', error);
  }
}

async function fetchPricesInternal(): Promise<Asset[]> {
  const pairs = PAIR_CONFIGS.map(config => config.pair);

  const [midenPrices, binanceStats] = await Promise.all([
    fetchAllMediansWithRetry(pairs),
    fetchMultiple24hStats(pairs),
  ]);

  const assets: Asset[] = PAIR_CONFIGS.map(({ pair, name, marketCap }) => {
    const price = midenPrices.get(pair)?.price ?? 0;
    const symbol = pair;
    
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
  
  return assets;
}

export async function GET() {
  try {
    const cacheKey = 'prices:all';
    
    const cached = getCached<Asset[]>(cacheKey);
    if (cached) {
      return Response.json(cached);
    }
    
    let pendingPromise = pendingRequests.get(cacheKey);
    
    if (!pendingPromise) {
      pendingPromise = fetchPricesInternal()
        .then(assets => {
          setCache(cacheKey, assets, CACHE_TTL);
          pendingRequests.delete(cacheKey);
          return assets;
        })
        .catch(error => {
          pendingRequests.delete(cacheKey);
          throw error;
        });
      
      pendingRequests.set(cacheKey, pendingPromise);
    }
    
    const assets = await pendingPromise;
    return Response.json(assets);
  } catch (error) {
    console.error('Prices API error:', error);
    return Response.json(
      { error: 'Failed to fetch prices', details: String(error) },
      { status: 500 }
    );
  }
}
