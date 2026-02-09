import { NextResponse } from 'next/server';
import { insertPriceHistory, type PriceHistoryRow } from '@/lib/db';

const PRAGMA_API_BASE = process.env.PRAGMA_API_BASE_URL || 'https://api.production.pragma.build/node/v1/onchain/history';
const PRAGMA_API_KEY = process.env.PRAGMA_API_KEY;
const NETWORK = 'starknet-mainnet';
const PAIRS = ['BTC/USD', 'ETH/USD'];

interface PragmaHistoryEntry {
  pair_id: string;
  timestamp: number;
  median_price: string;
  decimals: number;
  nb_sources_aggregated: number;
}

async function fetchHistoricalData(pair: string, startTime: number, endTime: number): Promise<PriceHistoryRow[]> {
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
      console.error(`Failed to fetch data for ${pair}: ${response.status} ${response.statusText}`, errorText);
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
    console.error(`Error fetching historical data for ${pair}:`, error);
    return [];
  }
}

export async function GET() {
  try {
    const endTime = Math.floor(Date.now() / 1000);
    const startTime = endTime - 86400;
    
    const results = await Promise.all(
      PAIRS.map(pair => fetchHistoricalData(pair, startTime, endTime))
    );
    
    const allRows = results.flat();
    
    if (allRows.length === 0) {
      return NextResponse.json(
        { error: 'No historical data fetched', message: 'All pairs failed to fetch data' },
        { status: 500 }
      );
    }
    
    const insertedCount = insertPriceHistory(allRows);
    
    const summary = PAIRS.map((pair, index) => ({
      pair,
      dataPoints: results[index].length,
    }));
    
    return NextResponse.json({
      success: true,
      totalInserted: insertedCount,
      totalDataPoints: allRows.length,
      summary,
      startTime: new Date(startTime * 1000).toISOString(),
    });
  } catch (error) {
    console.error('Seed API error:', error);
    return NextResponse.json(
      { error: 'Failed to seed historical data', details: String(error) },
      { status: 500 }
    );
  }
}
