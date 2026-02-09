"use client";

import type { Asset } from "@/types/asset";

interface AssetCardProps {
  asset: Asset;
  loading: boolean;
  priceChange: 'up' | 'down' | null;
  onClick?: () => void;
}

export function AssetCard({ asset, loading, priceChange, onClick }: AssetCardProps) {
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
    <div 
      onClick={onClick}
      className={`group bg-gradient-to-br from-surface to-surface-elevated border border-border rounded-xl p-7 hover:border-border-hover hover:shadow-2xl hover:shadow-black/50 hover:-translate-y-1 transition-all duration-500 animate-fade-in relative cursor-pointer overflow-hidden shimmer-overlay ${flashColor}`}
    >
      <div className="absolute inset-0 bg-gradient-to-br from-white/[0.02] to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none" />
      <div className="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-transparent via-primary/20 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500" />
      
      <div className="relative z-10">
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-4">
            <div className="w-14 h-14 rounded-xl bg-gradient-to-br from-surface-elevated to-background border border-border shadow-lg flex items-center justify-center group-hover:scale-110 transition-transform duration-300">
              <span className="text-text-primary font-bold text-lg tracking-tight">
                {asset.symbol.split('/')[0]}
              </span>
            </div>
            <div>
              <h3 className="font-bold text-xl text-text-primary tracking-tight">
                {asset.symbol}
              </h3>
              <p className="text-sm text-text-secondary mt-0.5">{asset.name}</p>
            </div>
          </div>
          
          <div className={`flex items-center gap-1.5 px-3.5 py-2 rounded-lg bg-black/20 backdrop-blur-sm border border-white/5 ${changeColor}`}>
            <span className="text-sm font-bold">{changeIcon}</span>
            <span className="text-sm font-bold tracking-tight">
              {Math.abs(asset.change24h).toFixed(2)}%
            </span>
          </div>
        </div>

        <div className="flex items-baseline gap-3">
          <p className={`text-5xl font-bold tracking-tighter transition-all duration-300 ${
            priceChange === 'up' ? 'text-primary scale-105' : 
            priceChange === 'down' ? 'text-danger scale-105' : 
            'text-text-primary'
          }`}>
            ${formatPrice(asset.price)}
          </p>
          {priceChange && (
            <span className={`text-3xl font-bold animate-bounce-subtle ${priceChange === 'up' ? 'text-primary' : 'text-danger'}`}>
              {priceChange === 'up' ? '↑' : '↓'}
            </span>
          )}
        </div>
      </div>

      {loading && (
        <div className="absolute inset-0 bg-surface/90 backdrop-blur-md rounded-xl flex items-center justify-center z-20">
          <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-primary"></div>
        </div>
      )}
    </div>
  );
}
