import { cleanupOldPrices, getDbStats } from './db';

const CLEANUP_INTERVAL_MS = 3600000; // 1 hour
const MAX_DATA_AGE_HOURS = 48; // Keep only last 48h

let cleanupIntervalId: NodeJS.Timeout | null = null;

/**
 * Start automatic cleanup of old price data
 * Runs every hour and removes data older than 48h
 */
export function startAutomaticCleanup(): void {
  if (cleanupIntervalId) {
    console.log('🧹 Cleanup already running');
    return;
  }

  console.log(`🧹 Starting automatic cleanup (every ${CLEANUP_INTERVAL_MS / 60000} minutes, keeping ${MAX_DATA_AGE_HOURS}h of data)`);

  cleanupIntervalId = setInterval(() => {
    try {
      const deleted = cleanupOldPrices(MAX_DATA_AGE_HOURS);
      
      if (deleted > 0) {
        const stats = getDbStats();
        console.log(`🧹 Cleaned up ${deleted} old price records. Database now has ${stats.totalRows} rows.`);
      }
    } catch (error) {
      console.error('❌ Failed to cleanup old prices:', error);
    }
  }, CLEANUP_INTERVAL_MS);

  // Also run cleanup immediately on startup
  try {
    const deleted = cleanupOldPrices(MAX_DATA_AGE_HOURS);
    if (deleted > 0) {
      console.log(`🧹 Initial cleanup: removed ${deleted} old records`);
    }
  } catch (error) {
    console.error('❌ Failed initial cleanup:', error);
  }
}

/**
 * Stop automatic cleanup
 */
export function stopAutomaticCleanup(): void {
  if (cleanupIntervalId) {
    clearInterval(cleanupIntervalId);
    cleanupIntervalId = null;
    console.log('🛑 Stopped automatic cleanup');
  }
}
