# Quick Start - Oracle Explorer

## Setup (première fois uniquement)

```bash
# Copier le fichier d'exemple
cp .env.example .env.local

# Les chemins par défaut devraient fonctionner si tu es dans oracle-explorer/
# Sinon, édite .env.local avec tes chemins
```

## Lancer avec les prix en live

### 1. Démarrer les publishers

Dans le répertoire racine du projet:

```bash
cd ..
./demo-publishers.sh
```

Ceci publie les prix BTC/USD, ETH/USD, SOL/USD toutes les ~15s depuis Binance et Bybit.

### 2. Lancer le frontend

Dans un **nouveau terminal**:

```bash
cd oracle-explorer
pnpm dev
```

### 3. Ouvrir

http://localhost:3000

## C'est tout! 🚀

Les prix se mettent à jour automatiquement toutes les 10s.

---

## Mode production

```bash
pnpm build
pnpm start
```

## Troubleshooting

**Pas de prix?**
- Vérifie que `demo-publishers.sh` tourne (Terminal 1)
- Attends 15-20s pour la première mise à jour

**Erreur "Failed to fetch median"?**
- Les publishers n'ont pas encore publié de données
- Attends 30s et refresh la page
