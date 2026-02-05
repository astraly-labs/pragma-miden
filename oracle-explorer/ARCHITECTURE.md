# Architecture - Oracle Explorer

## Comment le frontend accède aux prix on-chain

```
┌─────────────────────────────────────────────────────────────────┐
│                        FLOW COMPLET                             │
└─────────────────────────────────────────────────────────────────┘

1. Publishers (on-chain)
   ├─ Publisher1 publie BTC/USD, ETH/USD, SOL/USD → Miden testnet
   └─ Publisher2 publie BTC/USD, ETH/USD, SOL/USD → Miden testnet

2. Frontend (browser)
   └─ Fetch http://localhost:3000/api/prices

3. Next.js API Route (/app/api/prices/route.ts)
   ├─ Spawne pm-oracle-cli BATCH command (optimized):
   │  $ pm-oracle-cli median-batch BTC/USD ETH/USD SOL/USD --network testnet --json
   │
   ├─ Le CLI query l'oracle on-chain via RPC testnet
   ├─ Parse le JSON output: [{"pair":"BTC/USD","median":77985045000}, ...]
   ├─ Divise par 1_000_000 → 77985.045
   │
   └─ Combine avec metadata Binance (change24h, high/low)

4. Cache (10s TTL)
   └─ Évite de surcharger le testnet RPC

5. Retour au frontend
   └─ JSON: [{symbol: "BTC/USD", price: 77985.045, ...}]
```

## Code clé

### API Route spawne le CLI (Batch Optimized)

```typescript
// app/api/prices/route.ts
const { stdout } = await execAsync(
  `cd ${ORACLE_WORKSPACE} && ${CLI_PATH}/pm-oracle-cli median-batch BTC/USD ETH/USD SOL/USD --network testnet --json`
);

const results: MedianResult[] = JSON.parse(stdout);
results.forEach(({ pair, median }) => {
  priceMap.set(pair, median / 1_000_000);
});
```

### Le CLI query on-chain

```bash
# Batch command (optimized - 47% faster):
$ pm-oracle-cli median-batch BTC/USD ETH/USD SOL/USD --network testnet --json

→ Se connecte au testnet RPC (ONCE)
→ Lit le storage de l'oracle on-chain (ONCE)
→ Récupère les prix des 2 publishers (ONCE)
→ Loop: Calcule la médiane pour chaque pair
→ Output JSON: [{"pair":"BTC/USD","median":77985045000}, ...]
```

## Variables d'environnement

```bash
# .env.local
ORACLE_WORKSPACE_PATH=/path/to/.demo-workspaces/oracle
CLI_PATH=/path/to/target/release
```

Le workspace oracle contient:
- `pragma_miden.json` - Config de l'oracle
- `keystore/` - Clés de l'oracle
- `local-node/` - State local sync avec testnet

## Performance

### Before Optimization (3 sequential spawns)
- **Query on-chain**: ~1.5s per pair
- **3 pairs sequential**: ~4.68s total
- Each spawn syncs client + fetches oracle (~1.2s wasted overhead)

### After Batch Optimization
- **Batch query**: ~2.47s total
- **Improvement**: 47% faster (2.21s saved)
- Single sync + fetch, then loop through pairs
- **Cache**: 10s TTL
- **Frontend refresh**: 10s

→ Minimal load on testnet RPC thanks to cache + batch optimization

## Alternative sans spawn CLI

Une alternative serait de réécrire la logique du CLI directement en Node.js, mais:
- Complexe (besoin de reimplémenter la logique Miden)
- Le CLI est déjà optimisé et testé
- Spawn est suffisamment rapide pour une démo

Pour la production, on pourrait:
1. Compiler le CLI en WASM
2. Ou créer un backend Rust avec API HTTP
3. Ou utiliser un indexer off-chain
