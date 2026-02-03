"use client";

import type { Asset } from "@/types/asset";

interface AssetCardProps {
  asset: Asset;
  loading: boolean;
  priceChange: 'up' | 'down' | null;
}

export function AssetCard({ asset, loading, priceChange }: AssetCardProps) {
  const formatPrice = (price: number) => {
    if (price < 1) {
      return price.toFixed(6);
    } else if (price < 100) {
      return price.toFixed(4);
    } else {
      return price.toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      });
    }
  };

  const formatChange = (change: number) => {
    const sign = change >= 0 ? "+" : "";
    return `${sign}${change.toFixed(2)}%`;
  };

  const changeColor = asset.change24h >= 0 ? "text-primary" : "text-danger";
  const changeIcon = asset.change24h >= 0 ? "↑" : "↓";
  
  const flashColor = priceChange === 'up' 
    ? 'border-primary/50' 
    : priceChange === 'down' 
    ? 'border-danger/50' 
    : '';

  return (
    <div className={`bg-surface border border-border rounded-lg p-5 hover:border-border-hover transition-all duration-300 animate-fade-in relative ${flashColor}`}>
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-surface-elevated border border-border flex items-center justify-center">
            <span className="text-text-primary font-bold text-sm">
              {asset.symbol.split('/')[0]}
            </span>
          </div>
          <div>
            <h3 className="font-semibold text-base text-text-primary">
              {asset.symbol}
            </h3>
            <p className="text-xs text-text-muted">{asset.name}</p>
          </div>
        </div>
        
        <div className={`flex items-center gap-1 px-2 py-1 rounded ${changeColor}`}>
          <span className="text-xs font-bold">{changeIcon}</span>
          <span className="text-xs font-bold">
            {Math.abs(asset.change24h).toFixed(2)}%
          </span>
        </div>
      </div>

      <div className="flex items-baseline gap-2">
        <p className={`text-2xl font-bold text-text-primary transition-all duration-200 ${priceChange ? 'scale-105' : ''}`}>
          ${formatPrice(asset.price)}
        </p>
        {priceChange && (
          <span className={`text-lg font-bold ${priceChange === 'up' ? 'text-primary' : 'text-danger'}`}>
            {priceChange === 'up' ? '↑' : '↓'}
          </span>
        )}
      </div>

      {loading && (
        <div className="absolute inset-0 bg-surface/80 backdrop-blur-sm rounded-lg flex items-center justify-center">
          <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2 border-primary"></div>
        </div>
      )}
    </div>
  );
}
