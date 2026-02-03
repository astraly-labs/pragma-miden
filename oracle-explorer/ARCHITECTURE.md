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
   ├─ Spawne pm-oracle-cli pour chaque pair:
   │  $ pm-oracle-cli median BTC/USD --network testnet
   │  $ pm-oracle-cli median ETH/USD --network testnet
   │  $ pm-oracle-cli median SOL/USD --network testnet
   │
   ├─ Le CLI query l'oracle on-chain via RPC testnet
   ├─ Parse le stdout: "Median value: 77985045000"
   ├─ Divise par 1_000_000 → 77985.045
   │
   └─ Combine avec metadata Binance (change24h, high/low)

4. Cache (10s TTL)
   └─ Évite de surcharger le testnet RPC

5. Retour au frontend
   └─ JSON: [{symbol: "BTC/USD", price: 77985.045, ...}]
```

## Code clé

### API Route spawne le CLI

```typescript
// app/api/prices/route.ts
const { stdout } = await execAsync(
  `cd ${ORACLE_WORKSPACE} && ${CLI_PATH}/pm-oracle-cli median ${pair} --network testnet`
);

const match = stdout.match(/Median value: (\d+)/);
const price = parseInt(match[1]) / 1_000_000;
```

### Le CLI query on-chain

```bash
# Ce qui se passe sous le capot:
$ pm-oracle-cli median BTC/USD --network testnet

→ Se connecte au testnet RPC
→ Lit le storage de l'oracle on-chain
→ Récupère les prix des 2 publishers
→ Calcule la médiane
→ Output: "Median value: 77985045000"
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

- **Query on-chain**: ~1.5s par pair
- **3 pairs en parallèle**: ~4.5s total
- **Cache**: 10s TTL
- **Frontend refresh**: 10s

→ Peu de load sur le testnet RPC grâce au cache

## Alternative sans spawn CLI

Une alternative serait de réécrire la logique du CLI directement en Node.js, mais:
- Complexe (besoin de reimplémenter la logique Miden)
- Le CLI est déjà optimisé et testé
- Spawn est suffisamment rapide pour une démo

Pour la production, on pourrait:
1. Compiler le CLI en WASM
2. Ou créer un backend Rust avec API HTTP
3. Ou utiliser un indexer off-chain
