import { insertPriceHistory, getDbStats, type PriceHistoryRow } from './db';

const PRAGMA_API_BASE = process.env.PRAGMA_API_BASE_URL || 'https://api.production.pragma.build/node/v1/onchain/history';
const PRAGMA_API_KEY = process.env.PRAGMA_API_KEY;
const NETWORK = 'starknet-mainnet';
const PAIRS = ['BTC/USD', 'ETH/USD'];
const HISTORICAL_HOURS = 24;

interface PragmaHistoryEntry {
  pair_id: string;
  timestamp: number;
  median_price: string;
  decimals: number;
  nb_sources_aggregated: number;
}

async function fetchHistoricalDataForPair(
  pair: string, 
  startTime: number, 
  endTime: number
): Promise<PriceHistoryRow[]> {
  const [base, quote] = pair.split('/');
  const timestampRange = `${startTime},${endTime}`;
  const url = `${PRAGMA_API_BASE}/${base}/${quote}?network=${NETWORK}&timestamp=${timestampRange}&chunk_interval=1h`;
  
  try {
    const headers: Record<string, string> = {
      'Accept': 'application/json',
    };
    
    if (PRAGMA_API_KEY) {
      headers['x-api-key'] = PRAGMA_API_KEY;
    }
    
    const response = await fetch(url, { headers });
    
    if (!response.ok) {
      const errorText = await response.text();
      console.error(`Failed to fetch ${pair}: ${response.status}`, errorText);
      return [];
    }
    
    const data: PragmaHistoryEntry[] = await response.json();
    
    if (!Array.isArray(data)) {
      console.warn(`Invalid response format for ${pair}`);
      return [];
    }
    
    return data.map(point => ({
      pair,
      price: parseInt(point.median_price, 16) / Math.pow(10, point.decimals),
      decimals: point.decimals,
      timestamp: point.timestamp,
    }));
  } catch (error) {
    console.error(`Error fetching ${pair}:`, error);
    return [];
  }
}

export async function seedDatabaseIfEmpty(): Promise<void> {
  const stats = getDbStats();
  
  if (stats.totalRows > 0) {
    const oldestDate = stats.oldestTimestamp ? new Date(stats.oldestTimestamp * 1000).toISOString() : 'unknown';
    const newestDate = stats.newestTimestamp ? new Date(stats.newestTimestamp * 1000).toISOString() : 'unknown';
    console.log(`✅ Database already contains ${stats.totalRows} rows (${oldestDate} to ${newestDate})`);
    return;
  }
  
  if (!PRAGMA_API_KEY) {
    console.warn('⚠️ PRAGMA_API_KEY not set. Skipping historical data seeding.');
    console.warn('   Set PRAGMA_API_KEY in .env.local to enable automatic seeding.');
    return;
  }
  
  console.log(`📊 Database empty. Fetching ${HISTORICAL_HOURS}h of historical data from Pragma API...`);
  console.log(`   Seeding BTC/USD and ETH/USD only (other assets will have live data only)`);
  
  const endTime = Math.floor(Date.now() / 1000);
  const startTime = endTime - (HISTORICAL_HOURS * 3600);
  
  const results = await Promise.allSettled(
    PAIRS.map(pair => fetchHistoricalDataForPair(pair, startTime, endTime))
  );
  
  const successfulResults = results
    .filter((result): result is PromiseFulfilledResult<PriceHistoryRow[]> => result.status === 'fulfilled')
    .map(result => result.value);
  
  const allRows = successfulResults.flat();
  
  if (allRows.length === 0) {
    console.error('❌ No historical data fetched. All pairs failed.');
    return;
  }
  
  const inserted = insertPriceHistory(allRows);
  
  const summary = PAIRS.map((pair, index) => {
    const result = results[index];
    if (result.status === 'fulfilled') {
      return `${pair}: ${result.value.length} points`;
    } else {
      return `${pair}: failed`;
    }
  }).join(', ');
  
  console.log(`✅ Seeded ${inserted} historical data points`);
  console.log(`   Summary: ${summary}`);
}
