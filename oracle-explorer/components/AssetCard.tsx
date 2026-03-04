"use client";

import type { Asset } from "@/types/asset";

interface AssetCardProps {
  asset: Asset;
  loading: boolean;
  priceChange: "up" | "down" | null;
  onClick?: () => void;
}

export function AssetCard({ asset, loading, priceChange, onClick }: AssetCardProps) {
  const formatPrice = (price: number) => {
    if (price < 1) return price.toFixed(6);
    if (price < 100) return price.toFixed(4);
    return price.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  };

  const changePositive = asset.change24h >= 0;

  return (
    <div
      onClick={onClick}
      className={`card relative p-5 cursor-pointer animate-fade-in transition-all duration-200 hover:-translate-y-0.5 ${
        priceChange === "up" ? "border-[rgba(255,255,255,0.2)]" :
        priceChange === "down" ? "border-[rgba(239,68,68,0.2)]" : ""
      }`}
    >
      <div className="flex items-center justify-between mb-4">
        <span className="text-[rgba(255,255,255,0.35)] text-xs font-mono uppercase tracking-wider">
          {asset.symbol}
        </span>
        <span className={`text-xs font-mono ${changePositive ? "text-white" : "text-danger"}`}>
          {changePositive ? "+" : ""}{asset.change24h.toFixed(2)}%
        </span>
      </div>

      <div className="flex items-baseline gap-1.5">
        <span className="text-[rgba(255,255,255,0.25)] text-lg font-light">$</span>
        <p className={`text-2xl font-semibold tracking-tight transition-colors duration-200 ${
          priceChange === "up" ? "text-white" :
          priceChange === "down" ? "text-danger" :
          "text-white"
        }`}>
          {formatPrice(asset.price)}
        </p>
        {priceChange && (
          <span className={`text-sm animate-bounce-subtle ${priceChange === "up" ? "text-white" : "text-danger"}`}>
            {priceChange === "up" ? "\u2191" : "\u2193"}
          </span>
        )}
      </div>

      <p className="text-[rgba(255,255,255,0.25)] text-xs mt-2 font-mono">{asset.name}</p>

      {loading && (
        <div className="absolute inset-0 bg-[#111]/90 backdrop-blur-sm rounded-xl flex items-center justify-center z-20">
          <div className="w-5 h-5 border border-[rgba(255,255,255,0.2)] border-t-white rounded-full animate-spin"></div>
        </div>
      )}
    </div>
  );
}
