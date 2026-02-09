import Database from 'better-sqlite3';
import path from 'path';

const dbPath = path.join(process.cwd(), 'prices.db');

let db: Database.Database | null = null;

export function getDb(): Database.Database {
  if (!db) {
    db = new Database(dbPath);
    db.pragma('journal_mode = WAL');
    
    db.exec(`
      CREATE TABLE IF NOT EXISTS price_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        pair TEXT NOT NULL,
        price REAL NOT NULL,
        decimals INTEGER NOT NULL,
        timestamp INTEGER NOT NULL,
        created_at INTEGER NOT NULL DEFAULT (unixepoch()),
        UNIQUE(pair, timestamp)
      );
      
      CREATE INDEX IF NOT EXISTS idx_pair_timestamp 
      ON price_history(pair, timestamp DESC);
    `);
  }
  
  return db;
}

export interface PriceHistoryRow {
  pair: string;
  price: number;
  decimals: number;
  timestamp: number;
}

export function insertPriceHistory(rows: PriceHistoryRow[]): number {
  if (rows.length === 0) return 0;
  
  const db = getDb();
  const insert = db.prepare(`
    INSERT OR REPLACE INTO price_history (pair, price, decimals, timestamp)
    VALUES (?, ?, ?, ?)
  `);
  
  const insertMany = db.transaction((rows: PriceHistoryRow[]) => {
    for (const row of rows) {
      insert.run(row.pair, row.price, row.decimals, row.timestamp);
    }
  });
  
  insertMany(rows);
  return rows.length;
}

export function getPriceHistory(pair: string, startTime?: number, endTime?: number): PriceHistoryRow[] {
  const db = getDb();
  
  let query = 'SELECT pair, price, decimals, timestamp FROM price_history WHERE pair = ?';
  const params: any[] = [pair];
  
  if (startTime) {
    query += ' AND timestamp >= ?';
    params.push(startTime);
  }
  
  if (endTime) {
    query += ' AND timestamp <= ?';
    params.push(endTime);
  }
  
  query += ' ORDER BY timestamp ASC';
  
  const stmt = db.prepare(query);
  return stmt.all(...params) as PriceHistoryRow[];
}

export function getDbStats(): { totalRows: number; oldestTimestamp: number | null; newestTimestamp: number | null } {
  const db = getDb();
  
  const stats = db.prepare(`
    SELECT 
      COUNT(*) as totalRows,
      MIN(timestamp) as oldestTimestamp,
      MAX(timestamp) as newestTimestamp
    FROM price_history
  `).get() as { totalRows: number; oldestTimestamp: number | null; newestTimestamp: number | null };
  
  return stats;
}

export function cleanupOldPrices(olderThanHours: number = 48): number {
  const db = getDb();
  const cutoffTime = Math.floor(Date.now() / 1000) - (olderThanHours * 3600);
  
  const result = db.prepare('DELETE FROM price_history WHERE timestamp < ?').run(cutoffTime);
  
  return result.changes;
}

export interface Stats24h {
  change24h: number;
  high24h: number;
  low24h: number;
}

export function calculate24hStats(pair: string, currentPrice: number): Stats24h | null {
  const db = getDb();
  const now = Math.floor(Date.now() / 1000);
  const startTime24h = now - (24 * 60 * 60);
  
  const stats = db.prepare(`
    SELECT 
      MIN(price) as low24h,
      MAX(price) as high24h
    FROM price_history
    WHERE pair = ? AND timestamp >= ?
  `).get(pair, startTime24h) as { low24h: number | null; high24h: number | null } | undefined;
  
  if (!stats || stats.low24h === null || stats.high24h === null) {
    return null;
  }
  
  const price24hAgo = db.prepare(`
    SELECT price
    FROM price_history
    WHERE pair = ? AND timestamp >= ?
    ORDER BY timestamp ASC
    LIMIT 1
  `).get(pair, startTime24h) as { price: number } | undefined;
  
  if (!price24hAgo) {
    return null;
  }
  
  const change24h = ((currentPrice - price24hAgo.price) / price24hAgo.price) * 100;
  
  return {
    change24h,
    high24h: stats.high24h,
    low24h: stats.low24h,
  };
}
