"use client";

import { useState, useEffect, useRef } from "react";
import { AssetCard } from "@/components/AssetCard";
import { ChartModal } from "@/components/ChartModal";
import { Header } from "@/components/Header";
import { Footer } from "@/components/Footer";
import type { Asset } from "@/types/asset";
import { fetchPrices } from "@/lib/api";

const REFRESH_INTERVAL = 5000;

export default function Home() {
  const [assets, setAssets] = useState<Asset[]>([]);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);
  const [priceChanges, setPriceChanges] = useState<Record<string, 'up' | 'down' | null>>({});
  const previousPricesRef = useRef<Record<string, number>>({});
  const [selectedAsset, setSelectedAsset] = useState<Asset | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);

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

  const handleAssetClick = (asset: Asset) => {
    setSelectedAsset(asset);
    setIsModalOpen(true);
  };

  const handleCloseModal = () => {
    setIsModalOpen(false);
    setTimeout(() => setSelectedAsset(null), 300);
  };

  return (
    <main className="min-h-screen bg-gradient-to-br from-background via-background to-surface-elevated flex flex-col">
      <Header lastUpdate={lastUpdate} />

      <div className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-12">
        {loading && assets.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-96 gap-6">
            <div className="relative">
              <div className="animate-spin rounded-full h-16 w-16 border-t-2 border-b-2 border-primary shadow-lg shadow-primary/20"></div>
              <div className="absolute inset-0 rounded-full bg-primary/10 animate-pulse"></div>
            </div>
            <p className="text-text-secondary font-medium animate-pulse">Loading oracle data...</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-3 gap-6">
            {assets.map((asset, index) => (
              <div
                key={asset.symbol}
                style={{ animationDelay: `${index * 50}ms` }}
              >
                <AssetCard 
                  asset={asset} 
                  loading={loading}
                  priceChange={priceChanges[asset.symbol] || null}
                  onClick={() => handleAssetClick(asset)}
                />
              </div>
            ))}
          </div>
        )}
      </div>

      <Footer />

      <ChartModal 
        asset={selectedAsset}
        isOpen={isModalOpen}
        onClose={handleCloseModal}
      />
    </main>
  );
}
