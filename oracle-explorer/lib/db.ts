// In-memory price history store — replaces better-sqlite3 for standalone compatibility
// Data persists for the lifetime of the process (sufficient for 24h charts)

export interface PriceHistoryRow {
  pair: string;
  price: number;
  decimals: number;
  timestamp: number;
}

export interface Stats24h {
  change24h: number;
  high24h: number;
  low24h: number;
}

const MAX_ROWS_PER_PAIR = 2000;
const store = new Map<string, PriceHistoryRow[]>();

function getRows(pair: string): PriceHistoryRow[] {
  if (!store.has(pair)) store.set(pair, []);
  return store.get(pair)!;
}

export function insertPriceHistory(rows: PriceHistoryRow[]): number {
  if (rows.length === 0) return 0;

  for (const row of rows) {
    const existing = getRows(row.pair);
    const idx = existing.findIndex(r => r.timestamp === row.timestamp);
    if (idx !== -1) {
      existing[idx] = row;
    } else {
      existing.push(row);
    }
  }

  for (const [pair, rows] of store.entries()) {
    if (rows.length > MAX_ROWS_PER_PAIR) {
      rows.sort((a, b) => a.timestamp - b.timestamp);
      store.set(pair, rows.slice(rows.length - MAX_ROWS_PER_PAIR));
    }
  }

  return rows.length;
}

export function getPriceHistory(pair: string, startTime?: number, endTime?: number): PriceHistoryRow[] {
  let rows = [...getRows(pair)];
  if (startTime !== undefined) rows = rows.filter(r => r.timestamp >= startTime);
  if (endTime !== undefined) rows = rows.filter(r => r.timestamp <= endTime);
  return rows.sort((a, b) => a.timestamp - b.timestamp);
}

export function getDbStats(): { totalRows: number; oldestTimestamp: number | null; newestTimestamp: number | null } {
  let totalRows = 0;
  let oldest: number | null = null;
  let newest: number | null = null;

  for (const rows of store.values()) {
    totalRows += rows.length;
    for (const r of rows) {
      if (oldest === null || r.timestamp < oldest) oldest = r.timestamp;
      if (newest === null || r.timestamp > newest) newest = r.timestamp;
    }
  }

  return { totalRows, oldestTimestamp: oldest, newestTimestamp: newest };
}

export function cleanupOldPrices(olderThanHours = 48): number {
  const cutoff = Math.floor(Date.now() / 1000) - olderThanHours * 3600;
  let deleted = 0;

  for (const [pair, rows] of store.entries()) {
    const before = rows.length;
    store.set(pair, rows.filter(r => r.timestamp >= cutoff));
    deleted += before - store.get(pair)!.length;
  }

  return deleted;
}

export function calculate24hStats(pair: string, currentPrice: number): Stats24h | null {
  const now = Math.floor(Date.now() / 1000);
  const startTime24h = now - 24 * 3600;
  const rows = getPriceHistory(pair, startTime24h);

  if (rows.length === 0) return null;

  const low24h = Math.min(...rows.map(r => r.price));
  const high24h = Math.max(...rows.map(r => r.price));
  const price24hAgo = rows[0].price;
  const change24h = ((currentPrice - price24hAgo) / price24hAgo) * 100;

  return { change24h, high24h, low24h };
}
