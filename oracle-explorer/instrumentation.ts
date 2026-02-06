export async function register() {
  if (process.env.NEXT_RUNTIME === 'nodejs') {
    console.log('🚀 Initializing Oracle Explorer...');
    
    if (process.env.NODE_ENV === 'development') {
      const fs = await import('fs');
      const path = await import('path');
      const dbPath = path.join(process.cwd(), 'prices.db');
      const dbWalPath = path.join(process.cwd(), 'prices.db-wal');
      const dbShmPath = path.join(process.cwd(), 'prices.db-shm');
      
      let deleted = false;
      
      if (fs.existsSync(dbPath)) {
        fs.unlinkSync(dbPath);
        deleted = true;
      }
      
      if (fs.existsSync(dbWalPath)) {
        fs.unlinkSync(dbWalPath);
        deleted = true;
      }
      
      if (fs.existsSync(dbShmPath)) {
        fs.unlinkSync(dbShmPath);
        deleted = true;
      }
      
      if (deleted) {
        console.log('🗑️  Deleted old database (dev mode)');
      }
    }
    
    const { seedDatabaseIfEmpty } = await import('./lib/db-seed');
    const { startAutomaticCleanup } = await import('./lib/db-cleanup');
    
    await seedDatabaseIfEmpty();
    
    startAutomaticCleanup();
    
    console.log('✅ Oracle Explorer initialized');
  }
}
