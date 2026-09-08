/**
 * Exchange market data for display-only metadata in the Oracle Explorer:
 * 24h stats (change%, high, low) and the 24h chart fallback. Which exchange
 * serves a pair is declared once, in lib/faucet-config.ts (`market`).
 */

import type { PriceHistoryRow } from '@/lib/db';
import { PAIR_TO_MARKET, type MarketRef } from '@/lib/faucet-config';

export interface Stats24h {
  change24h: number;
  high24h: number;
  low24h: number;
}

const HISTORY_RANGE_MS = 24 * 60 * 60 * 1000;
const STATS_REVALIDATE_S = 10;
const HISTORY_REVALIDATE_S = 300;

async function fetchJson(url: string, revalidate: number): Promise<any | null> {
  const response = await fetch(url, { next: { revalidate } });
  if (!response.ok) {
    console.error(`Market API error ${response.status} for ${url}`);
    return null;
  }
  return response.json();
}

async function fetchStats(market: MarketRef): Promise<Stats24h | null> {
  switch (market.venue) {
    case 'binance': {
      const data = await fetchJson(
        `https://api.binance.com/api/v3/ticker/24hr?symbol=${market.symbol}`,
        STATS_REVALIDATE_S
      );
      if (!data) return null;
      return {
        change24h: parseFloat(data.priceChangePercent),
        high24h: parseFloat(data.highPrice),
        low24h: parseFloat(data.lowPrice),
      };
    }
    case 'kucoin': {
      const data = await fetchJson(
        `https://api.kucoin.com/api/v1/market/stats?symbol=${market.symbol}`,
        STATS_REVALIDATE_S
      );
      const ticker = data?.data;
      if (!ticker) return null;
      return {
        change24h: parseFloat(ticker.changeRate) * 100,
        high24h: parseFloat(ticker.high),
        low24h: parseFloat(ticker.low),
      };
    }
  }
}

// 30-minute close prices over the last 24h, oldest first.
async function fetchHistory(pair: string, market: MarketRef): Promise<PriceHistoryRow[]> {
  const endMs = Date.now();
  const startMs = endMs - HISTORY_RANGE_MS;
  const row = (timestamp: number, price: number): PriceHistoryRow => ({
    pair,
    price,
    decimals: 6,
    timestamp,
  });

  switch (market.venue) {
    case 'binance': {
      const data = await fetchJson(
        `https://api.binance.com/api/v3/klines?symbol=${market.symbol}&interval=30m&startTime=${startMs}&endTime=${endMs}`,
        HISTORY_REVALIDATE_S
      );
      if (!Array.isArray(data)) return [];
      // [openTime(ms), open, high, low, close, ...], oldest first
      return data.map((c: (string | number)[]) =>
        row(Math.floor(Number(c[0]) / 1000), parseFloat(String(c[4])))
      );
    }
    case 'kucoin': {
      const data = await fetchJson(
        `https://api.kucoin.com/api/v1/market/candles?type=30min&symbol=${market.symbol}&startAt=${Math.floor(startMs / 1000)}&endAt=${Math.floor(endMs / 1000)}`,
        HISTORY_REVALIDATE_S
      );
      const candles = data?.data;
      if (!Array.isArray(candles)) return [];
      // [time(s), open, close, high, low, ...], newest first
      return candles
        .map((c: string[]) => row(parseInt(c[0], 10), parseFloat(c[2])))
        .sort((a, b) => a.timestamp - b.timestamp);
    }
  }
}

export async function fetch24hStats(pair: string): Promise<Stats24h | null> {
  const market = PAIR_TO_MARKET.get(pair);
  if (!market) {
    console.warn(`No market mapping for pair: ${pair}`);
    return null;
  }
  try {
    return await fetchStats(market);
  } catch (error) {
    console.error(`Error fetching 24h stats for ${pair}:`, error);
    return null;
  }
}

export async function fetchMultiple24hStats(pairs: string[]): Promise<Map<string, Stats24h>> {
  const results = await Promise.all(
    pairs.map(async (pair) => ({ pair, stats: await fetch24hStats(pair) }))
  );
  const statsMap = new Map<string, Stats24h>();
  results.forEach(({ pair, stats }) => {
    if (stats) statsMap.set(pair, stats);
  });
  return statsMap;
}

export async function fetchMarketHistory(
  pair: string
): Promise<{ source: string; data: PriceHistoryRow[] }> {
  const market = PAIR_TO_MARKET.get(pair);
  if (!market) return { source: 'none', data: [] };
  try {
    return { source: market.venue, data: await fetchHistory(pair, market) };
  } catch (error) {
    console.error(`Error fetching ${market.venue} history for ${pair}:`, error);
    return { source: market.venue, data: [] };
  }
}
