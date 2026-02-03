"use client";

import { useState, useEffect, useRef } from "react";
import { AssetCard } from "@/components/AssetCard";
import { Header } from "@/components/Header";
import type { Asset } from "@/types/asset";
import { fetchPrices } from "@/lib/api";

const REFRESH_INTERVAL = 5000;

export default function Home() {
  const [assets, setAssets] = useState<Asset[]>([]);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);
  const [priceChanges, setPriceChanges] = useState<Record<string, 'up' | 'down' | null>>({});
  const previousPricesRef = useRef<Record<string, number>>({});

  useEffect(() => {
    loadPrices();
    const interval = setInterval(loadPrices, REFRESH_INTERVAL);
    return () => clearInterval(interval);
  }, []);

  const loadPrices = async () => {
    try {
      const data = await fetchPrices();
      
      const uniqueAssets = data.reduce((acc, asset) => {
        if (!acc[asset.symbol]) {
          acc[asset.symbol] = asset;
        }
        return acc;
      }, {} as Record<string, Asset>);
      
      const deduplicatedData = Object.values(uniqueAssets);
      
      const changes: Record<string, 'up' | 'down' | null> = {};
      deduplicatedData.forEach(asset => {
        const prevPrice = previousPricesRef.current[asset.symbol];
        if (prevPrice !== undefined) {
          if (asset.price > prevPrice) {
            changes[asset.symbol] = 'up';
          } else if (asset.price < prevPrice) {
            changes[asset.symbol] = 'down';
          } else {
            changes[asset.symbol] = null;
          }
        }
        previousPricesRef.current[asset.symbol] = asset.price;
      });
      
      setPriceChanges(changes);
      setTimeout(() => setPriceChanges({}), 600);
      
      setAssets(deduplicatedData);
      setLastUpdate(new Date());
      setLoading(false);
    } catch (error) {
      console.error("Failed to fetch prices:", error);
      setLoading(false);
    }
  };

  return (
    <main className="min-h-screen">
      <Header lastUpdate={lastUpdate} />

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {loading && assets.length === 0 ? (
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary"></div>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {assets.map((asset) => (
              <AssetCard 
                key={asset.symbol} 
                asset={asset} 
                loading={loading}
                priceChange={priceChanges[asset.symbol] || null}
              />
            ))}
          </div>
        )}
      </div>
    </main>
  );
}
