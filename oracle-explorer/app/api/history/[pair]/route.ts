import { NextRequest, NextResponse } from 'next/server';
import { getPriceHistory, type PriceHistoryRow } from '@/lib/db';

interface RouteParams {
  params: {
    pair: string;
  };
}

const TIME_RANGE_SECONDS = 24 * 60 * 60;
const DOWNSAMPLE_INTERVAL_SECONDS = 30 * 60;

const BYBIT_SYMBOL_MAP: Record<string, string> = {
  'BTC/USD': 'BTCUSDT',
  'ETH/USD': 'ETHUSDT',
  'SOL/USD': 'SOLUSDT',
  'BNB/USD': 'BNBUSDT',
  'XRP/USD': 'XRPUSDT',
  'HYPE/USD': 'HYPEUSDT',
  'POL/USD': 'POLUSDT',
};

async function fetchBybitHistory(pair: string): Promise<PriceHistoryRow[]> {
  const symbol = BYBIT_SYMBOL_MAP[pair];
  if (!symbol) {
    return [];
  }

  try {
    const endTime = Date.now();
    const startTime = endTime - (TIME_RANGE_SECONDS * 1000);
    
    const response = await fetch(
      `https://api.bybit.com/v5/market/kline?category=spot&symbol=${symbol}&interval=30&start=${startTime}&end=${endTime}`,
      { next: { revalidate: 300 } }
    );

    if (!response.ok) {
      console.error(`Bybit API error: ${response.status}`);
      return [];
    }

    const data = await response.json();
    
    if (!data.result?.list) {
      return [];
    }

    return data.result.list.map((candle: string[]) => ({
      pair,
      price: parseFloat(candle[4]),
      decimals: 6,
      timestamp: Math.floor(parseInt(candle[0]) / 1000),
    })).reverse();
  } catch (error) {
    console.error(`Error fetching Bybit history for ${pair}:`, error);
    return [];
  }
}

function downsampleData(data: PriceHistoryRow[], intervalSeconds: number): PriceHistoryRow[] {
  if (intervalSeconds === 0 || data.length === 0) return data;
  
  const buckets = new Map<number, PriceHistoryRow[]>();
  
  data.forEach(point => {
    const bucketKey = Math.floor(point.timestamp / intervalSeconds) * intervalSeconds;
    if (!buckets.has(bucketKey)) {
      buckets.set(bucketKey, []);
    }
    buckets.get(bucketKey)!.push(point);
  });
  
  return Array.from(buckets.entries())
    .map(([bucketTimestamp, points]) => {
      const avgPrice = points.reduce((sum, p) => sum + p.price, 0) / points.length;
      return {
        pair: points[0].pair,
        price: avgPrice,
        decimals: points[0].decimals,
        timestamp: bucketTimestamp,
      };
    })
    .sort((a, b) => a.timestamp - b.timestamp);
}

const PAIRS_WITH_FULL_HISTORY = ['BTC/USD', 'ETH/USD'];
const MIN_REQUIRED_POINTS = 10;

export async function GET(
  request: NextRequest,
  { params }: RouteParams
) {
  try {
    const pair = decodeURIComponent(params.pair);
    
    const now = Math.floor(Date.now() / 1000);
    const startTime = now - TIME_RANGE_SECONDS;
    
    let history = getPriceHistory(pair, startTime, now);
    let source = 'oracle';
    
    if (!PAIRS_WITH_FULL_HISTORY.includes(pair) || history.length < MIN_REQUIRED_POINTS) {
      const bybitHistory = await fetchBybitHistory(pair);
      if (bybitHistory.length > history.length) {
        history = bybitHistory;
        source = 'bybit';
      }
    }
    
    if (DOWNSAMPLE_INTERVAL_SECONDS > 0 && history.length > 0) {
      history = downsampleData(history, DOWNSAMPLE_INTERVAL_SECONDS);
    }
    
    return NextResponse.json({
      pair,
      data: history,
      count: history.length,
      source,
    });
  } catch (error) {
    console.error(`History API error for pair ${params.pair}:`, error);
    return NextResponse.json(
      { error: 'Failed to fetch price history', details: String(error) },
      { status: 500 }
    );
  }
}
