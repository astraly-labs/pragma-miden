import { exec } from 'child_process';
import { promisify } from 'util';
import { getCached, setCache } from '@/lib/cache';
import { fetchMultiple24hStats } from '@/lib/binance-api';
import { insertPriceHistory } from '@/lib/db';
import type { Asset } from '@/types/asset';
import { FAUCET_CONFIGS, FAUCET_ID_TO_PAIR, PAIR_TO_FAUCET_ID } from '@/lib/faucet-config';

const execAsync = promisify(exec);

const NETWORK = process.env.NETWORK || 'testnet';
const ORACLE_WORKSPACE = process.env.ORACLE_WORKSPACE_PATH || '';
const CLI_PATH = process.env.CLI_PATH || '';
const CACHE_TTL = 10000; // Increased from 4s to 10s to reduce spawn frequency
const MAX_RETRIES = 3;

// Lock to prevent multiple concurrent CLI spawns (cache stampede prevention)
const pendingRequests = new Map<string, Promise<Asset[]>>();

interface MedianResult {
  faucet_id: string;
  is_tracked: boolean;
  median: number;
  amount: number;
}

async function fetchAllMediansWithRetry(faucetIds: string[]): Promise<Map<string, number>> {
  for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
    try {
      const { stdout } = await execAsync(
        `cd ${ORACLE_WORKSPACE} && ${CLI_PATH}/pm-oracle-cli median-batch ${faucetIds.join(' ')} --network ${NETWORK} --json`,
        { timeout: 30000 }
      );
      
      const lines = stdout.trim().split('\n');
      const jsonLine = lines.find(line => line.startsWith('['));
      
      if (jsonLine) {
        const results: MedianResult[] = JSON.parse(jsonLine);
        const priceMap = new Map<string, number>();
        
        results.forEach(({ faucet_id, is_tracked, median }) => {
          if (!is_tracked) return;
          
          const pair = FAUCET_ID_TO_PAIR.get(faucet_id);
          if (pair) {
            priceMap.set(pair, median / 1_000_000);
          }
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

async function fetchPricesInternal(): Promise<Asset[]> {
  const faucetIds = FAUCET_CONFIGS.map(config => config.faucetId);
  const pairs = FAUCET_CONFIGS.map(config => config.pair);
  
  const [midenPrices, binanceStats] = await Promise.all([
    fetchAllMediansWithRetry(faucetIds),
    fetchMultiple24hStats(pairs),
  ]);

  const assets: Asset[] = FAUCET_CONFIGS.map(({ pair, name, marketCap }) => {
    const price = midenPrices.get(pair) || 0;
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
