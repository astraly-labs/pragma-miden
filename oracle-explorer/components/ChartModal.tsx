"use client";

import { useEffect, useState } from 'react';
import { PriceChart } from './PriceChart';
import type { Asset } from '@/types/asset';

interface ChartModalProps {
  asset: Asset | null;
  isOpen: boolean;
  onClose: () => void;
}

interface PriceDataPoint {
  timestamp: number;
  price: number;
}

export function ChartModal({ asset, isOpen, onClose }: ChartModalProps) {
  const [historyData, setHistoryData] = useState<PriceDataPoint[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen || !asset) {
      setHistoryData([]);
      setError(null);
      return;
    }

    const fetchHistory = async () => {
      setLoading(true);
      setError(null);
      
      try {
        const response = await fetch(`/api/history/${encodeURIComponent(asset.symbol)}`);
        
        if (!response.ok) {
          throw new Error('Failed to fetch price history');
        }
        
        const data = await response.json();
        setHistoryData(data.data || []);
      } catch (err) {
        console.error('Error fetching price history:', err);
        setError('Failed to load price history');
        setHistoryData([]);
      } finally {
        setLoading(false);
      }
    };

    fetchHistory();
  }, [isOpen, asset]);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };

    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [isOpen, onClose]);

  if (!isOpen || !asset) return null;

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

  return (
    <>
      <div 
        className="fixed inset-0 bg-black/80 backdrop-blur-md z-40 animate-fade-in"
        onClick={onClose}
      />
      
      <div className="fixed inset-x-0 bottom-0 md:inset-0 md:flex md:items-center md:justify-center z-50 animate-slide-up p-4">
        <div 
          className="bg-gradient-to-br from-surface via-surface-elevated to-surface border md:border-2 border-border rounded-t-3xl md:rounded-3xl w-full md:max-w-5xl md:max-h-[90vh] overflow-hidden shadow-2xl shadow-black/60"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="p-8 border-b border-border/50 flex items-center justify-between bg-gradient-to-r from-black/20 to-transparent">
            <div className="flex items-center gap-5">
              <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-surface-elevated to-background border-2 border-border shadow-xl flex items-center justify-center">
                <span className="text-text-primary font-bold text-xl tracking-tight">
                  {asset.symbol.split('/')[0]}
                </span>
              </div>
              <div>
                <h2 className="text-3xl font-bold text-text-primary tracking-tight">{asset.symbol}</h2>
                <p className="text-sm text-text-secondary mt-1 font-medium">{asset.name}</p>
              </div>
            </div>
            
            <div className="flex items-center gap-6">
              <div className="text-right">
                <p className="text-4xl font-bold text-text-primary tracking-tighter">
                  ${formatPrice(asset.price)}
                </p>
                <p className={`text-sm font-bold mt-1 flex items-center justify-end gap-1.5 ${asset.change24h >= 0 ? 'text-primary' : 'text-danger'}`}>
                  <span>{asset.change24h >= 0 ? '↑' : '↓'}</span>
                  <span>{Math.abs(asset.change24h).toFixed(2)}%</span>
                  <span className="text-text-muted font-normal">(24h)</span>
                </p>
              </div>
              
              <button
                onClick={onClose}
                className="p-3 hover:bg-white/5 rounded-xl transition-all duration-300 hover:scale-110 group"
                aria-label="Close modal"
              >
                <svg className="w-6 h-6 text-text-muted group-hover:text-text-primary transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>
          
          <div className="p-8">
            <div className="flex items-center justify-between mb-8">
              <h3 className="text-xl font-bold text-text-primary tracking-tight">Price Chart</h3>
              <div className="text-xs text-text-secondary font-mono px-3 py-1.5 rounded-lg bg-black/20 border border-white/5">
                24H Data
              </div>
            </div>
            
            {loading && (
              <div className="w-full h-[450px] flex items-center justify-center bg-black/10 rounded-2xl border border-border/50">
                <div className="animate-spin rounded-full h-14 w-14 border-t-2 border-b-2 border-primary shadow-lg shadow-primary/20"></div>
              </div>
            )}
            
            {error && !loading && (
              <div className="w-full h-[450px] flex items-center justify-center bg-black/10 rounded-2xl border border-border/50">
                <p className="text-danger font-semibold">{error}</p>
              </div>
            )}
            
            {!loading && !error && historyData.length === 0 && (
              <div className="w-full h-[450px] flex flex-col items-center justify-center gap-6 bg-black/10 rounded-2xl border border-border/50">
                <div className="text-center space-y-3">
                  <p className="text-text-secondary text-lg font-semibold">No historical data available</p>
                  <p className="text-sm text-text-muted">Unable to load chart data for {asset.symbol}</p>
                  <div className="mt-6 px-4 py-2 rounded-lg bg-primary/10 border border-primary/20">
                    <p className="text-sm text-text-secondary">Live price updates are still active</p>
                  </div>
                </div>
              </div>
            )}
            
            {!loading && !error && historyData.length > 0 && (
              <div className="bg-black/10 rounded-2xl border border-border/50 p-6">
                <PriceChart data={historyData} pair={asset.symbol} />
              </div>
            )}
          </div>
        </div>
      </div>
    </>
  );
}
