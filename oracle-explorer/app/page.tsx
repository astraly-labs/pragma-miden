"use client";

import { useState, useEffect, useRef } from "react";
import { AssetCard } from "@/components/AssetCard";
import { ChartModal } from "@/components/ChartModal";
import { Header } from "@/components/Header";
import { Footer } from "@/components/Footer";
import type { Asset } from "@/types/asset";
import { fetchPrices } from "@/lib/api";

const REFRESH_INTERVAL = 10000;

const FAUCET_IDS = [
  { id: "1:0", pair: "BTC/USD" },
  { id: "2:0", pair: "ETH/USD" },
  { id: "3:0", pair: "SOL/USD" },
  { id: "4:0", pair: "BNB/USD" },
  { id: "5:0", pair: "XRP/USD" },
  { id: "6:0", pair: "HYPE/USD" },
  { id: "7:0", pair: "POL/USD" },
];

const CONTRACTS = [
  { name: "Oracle", address: "mtst1ar2frsv7kjz2gqzt2mt74d0xlyen8re3", url: "https://testnet.midenscan.com/account/mtst1arfh7akzc9m0wqz8m9a8xyup85g6ls32" },
  { name: "Publisher 1", address: "mtst1aqwdujtul020gqz8dlc6v00lgunczddf", url: "https://testnet.midenscan.com/account/mtst1arm5hzrpf5sg2qpry2a9r4w6f50rgfj4" },
  { name: "Publisher 2", address: "mtst1apkfkmest8g2sqq6qct5jt6s9s7834t6", url: "https://testnet.midenscan.com/account/mtst1ar4ucrw059sdvqzfekvvvt03dgs229pc" },
];

const PUBLISHER_STEPS = [
  { title: "Build the CLI tools", code: "cargo build --release" },
  { title: "Initialize your publisher account", code: "./target/release/pm-publisher-cli init" },
  { title: "Request registration", description: "Send your publisher ID to the Oracle administrator", code: "./target/release/pm-oracle-cli register-publisher YOUR_PUBLISHER_ID" },
  { title: "Start publishing prices", code: "./target/release/pm-publisher-cli publish 1:0 98179840000 6 1738593825", note: "Where 1:0 = BTC/USD, price has 6 decimal places" },
];

function CodeBlock({ code }: { code: string }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = async () => {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };
  return (
    <div className="code-block relative group mt-3">
      <button
        onClick={handleCopy}
        className="absolute top-2.5 right-2.5 p-1.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[rgba(255,255,255,0.08)] transition-all"
        aria-label="Copy"
      >
        {copied ? (
          <svg className="w-3.5 h-3.5 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
          </svg>
        ) : (
          <svg className="w-3.5 h-3.5 text-[rgba(255,255,255,0.3)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
          </svg>
        )}
      </button>
      <pre className="p-4 overflow-x-auto text-sm leading-relaxed">
        <code className="text-[#a8b3cf]">{code}</code>
      </pre>
    </div>
  );
}

export default function Home() {
  const [assets, setAssets] = useState<Asset[]>([]);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);
  const [priceChanges, setPriceChanges] = useState<Record<string, "up" | "down" | null>>({});
  const previousPricesRef = useRef<Record<string, number>>({});
  const [selectedAsset, setSelectedAsset] = useState<Asset | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [consumerTab, setConsumerTab] = useState<"single" | "batch">("single");

  useEffect(() => {
    loadPrices();
    const interval = setInterval(loadPrices, REFRESH_INTERVAL);
    return () => clearInterval(interval);
  }, []);

  const loadPrices = async () => {
    try {
      const data = await fetchPrices();
      const uniqueAssets = data.reduce((acc, asset) => {
        if (!acc[asset.symbol]) acc[asset.symbol] = asset;
        return acc;
      }, {} as Record<string, Asset>);
      const deduplicatedData = Object.values(uniqueAssets);

      const changes: Record<string, "up" | "down" | null> = {};
      deduplicatedData.forEach((asset) => {
        const prev = previousPricesRef.current[asset.symbol];
        if (prev !== undefined) {
          changes[asset.symbol] = asset.price > prev ? "up" : asset.price < prev ? "down" : null;
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

  const handleAssetClick = (asset: Asset) => { setSelectedAsset(asset); setIsModalOpen(true); };
  const handleCloseModal = () => { setIsModalOpen(false); setTimeout(() => setSelectedAsset(null), 300); };

  return (
    <main className="min-h-screen bg-[#080808] flex flex-col">
      <Header lastUpdate={lastUpdate} />

      {/* Hero */}
      <section className="relative overflow-hidden">
        <div className="relative max-w-6xl mx-auto px-6 lg:px-8 pt-24 pb-20 text-center">
          <p className="text-[rgba(255,255,255,0.35)] text-xs font-mono uppercase tracking-[0.2em] mb-6">Pragma Oracle · Miden</p>
          <h1 className="h1 text-white">
            Provable price feeds<br />
            <span className="text-[rgba(255,255,255,0.45)]">for Polygon Miden</span>
          </h1>
          <p className="text-[rgba(255,255,255,0.4)] text-base max-w-xl mx-auto mt-6 leading-relaxed">
            Real-time oracle data aggregated from multiple publishers,<br className="hidden sm:block" /> available natively on the Miden rollup.
          </p>
          <div className="flex flex-col sm:flex-row items-center justify-center gap-3 mt-10">
            <a href="#prices" className="btn-primary">View Live Prices</a>
            <a href="https://docs.pragma.build" target="_blank" rel="noopener noreferrer" className="btn-outline">Read the Docs</a>
          </div>
        </div>
      </section>

      <div className="max-w-6xl w-full mx-auto px-6 lg:px-8">
        <hr className="divider" />

        {/* Live Prices */}
        <section id="prices" className="py-16">
          <p className="text-[rgba(255,255,255,0.3)] text-xs font-mono uppercase tracking-[0.2em] mb-3">Live Feeds</p>
          <h2 className="h2 text-white mb-2">Real-time oracle prices</h2>
          <p className="text-[rgba(255,255,255,0.35)] text-sm mb-10">Updated every 10 seconds &middot; On-chain Miden data</p>

          {loading && assets.length === 0 ? (
            <div className="flex items-center justify-center h-48 gap-3">
              <div className="w-4 h-4 border border-[rgba(255,255,255,0.2)] border-t-white rounded-full animate-spin"></div>
              <span className="text-[rgba(255,255,255,0.3)] text-sm font-mono">Fetching oracle data...</span>
            </div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3">
              {assets.map((asset, index) => (
                <div key={asset.symbol} style={{ animationDelay: `${index * 40}ms` }}>
                  <AssetCard asset={asset} loading={loading} priceChange={priceChanges[asset.symbol] || null} onClick={() => handleAssetClick(asset)} />
                </div>
              ))}
            </div>
          )}
        </section>

        <hr className="divider" />

        {/* Publisher Integration */}
        <section className="py-16">
          <p className="text-[rgba(255,255,255,0.3)] text-xs font-mono uppercase tracking-[0.2em] mb-3">Publisher Integration</p>
          <h2 className="h2 text-white mb-2">Become a price publisher</h2>
          <p className="text-[rgba(255,255,255,0.35)] text-sm mb-12 max-w-xl">Push price data to the Miden oracle and contribute to decentralized aggregation.</p>

          <div className="space-y-10 max-w-2xl">
            {PUBLISHER_STEPS.map((step, i) => (
              <div key={i} className="flex gap-6">
                <div className="flex-shrink-0 pt-0.5">
                  <span className="text-[rgba(255,255,255,0.2)] font-mono text-sm">{String(i + 1).padStart(2, "0")}</span>
                </div>
                <div className="flex-1 min-w-0">
                  <h4 className="text-white font-medium text-sm mb-1">{step.title}</h4>
                  {step.description && <p className="text-[rgba(255,255,255,0.35)] text-xs mb-1">{step.description}</p>}
                  <CodeBlock code={step.code} />
                  {step.note && <p className="text-[rgba(255,255,255,0.2)] text-xs mt-2 font-mono">{step.note}</p>}
                </div>
              </div>
            ))}
          </div>

          <a href="https://github.com/astraly-labs/pragma-miden#integrate-as-publisher" target="_blank" rel="noopener noreferrer" className="btn-outline mt-10 gap-2">
            Read full docs
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14 5l7 7m0 0l-7 7m7-7H3" />
            </svg>
          </a>
        </section>

        <hr className="divider" />

        {/* Consumer Integration */}
        <section className="py-16">
          <p className="text-[rgba(255,255,255,0.3)] text-xs font-mono uppercase tracking-[0.2em] mb-3">Consumer Integration</p>
          <h2 className="h2 text-white mb-2">Query oracle data</h2>
          <p className="text-[rgba(255,255,255,0.35)] text-sm mb-10 max-w-xl">Single asset or batch queries with the CLI. 47% faster batch via single sync.</p>

          <div className="max-w-2xl">
            <div className="flex gap-0 mb-4 border-b border-[rgba(255,255,255,0.08)]">
              {(["single", "batch"] as const).map((tab) => (
                <button
                  key={tab}
                  onClick={() => setConsumerTab(tab)}
                  className={`px-4 py-2.5 text-sm capitalize transition-all border-b-2 -mb-px ${
                    consumerTab === tab
                      ? "text-white border-white"
                      : "text-[rgba(255,255,255,0.35)] border-transparent hover:text-[rgba(255,255,255,0.6)]"
                  }`}
                >
                  {tab === "single" ? "Single Asset" : "Batch Query"}
                </button>
              ))}
            </div>

            {consumerTab === "single" ? (
              <CodeBlock code={`./target/release/pm-oracle-cli median 1:0 --network testnet\n# Output: Median value: 76436215000`} />
            ) : (
              <CodeBlock code={`./target/release/pm-oracle-cli median-batch 1:0 2:0 3:0 --network testnet --json\n# Output: [{"faucet_id":"1:0","is_tracked":true,"median":76436215000},...]`} />
            )}

            <p className="text-[rgba(255,255,255,0.25)] text-xs mt-4 font-mono leading-relaxed">
              Returns: <span className="text-[rgba(255,255,255,0.4)]">is_tracked</span> &middot; <span className="text-[rgba(255,255,255,0.4)]">median_price</span> (6 decimals) &middot; <span className="text-[rgba(255,255,255,0.4)]">amount</span> (optional)
            </p>
          </div>

          <a href="https://github.com/astraly-labs/pragma-miden#integrate-as-consumer" target="_blank" rel="noopener noreferrer" className="btn-outline mt-10 gap-2">
            Read full docs
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14 5l7 7m0 0l-7 7m7-7H3" />
            </svg>
          </a>
        </section>

        <hr className="divider" />

        {/* Deployed Contracts */}
        <section className="py-16">
          <p className="text-[rgba(255,255,255,0.3)] text-xs font-mono uppercase tracking-[0.2em] mb-3">Deployments</p>
          <h2 className="h2 text-white mb-10">Testnet contracts</h2>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            {CONTRACTS.map((contract) => (
              <a key={contract.name} href={contract.url} target="_blank" rel="noopener noreferrer" className="card p-5 group block">
                <div className="flex items-center justify-between mb-3">
                  <span className="text-white text-sm font-medium">{contract.name}</span>
                  <svg className="w-3.5 h-3.5 text-[rgba(255,255,255,0.2)] group-hover:text-[rgba(255,255,255,0.6)] transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                  </svg>
                </div>
                <p className="text-[rgba(255,255,255,0.25)] text-xs font-mono truncate">{contract.address}</p>
              </a>
            ))}
          </div>
        </section>

        <hr className="divider" />

        {/* Faucet IDs */}
        <section className="py-16">
          <p className="text-[rgba(255,255,255,0.3)] text-xs font-mono uppercase tracking-[0.2em] mb-6">Faucet IDs</p>
          <div className="flex flex-wrap gap-3">
            {FAUCET_IDS.map((faucet) => (
              <div key={faucet.id} className="flex items-center gap-2 px-3 py-2 border border-[rgba(255,255,255,0.08)] rounded">
                <span className="text-[rgba(255,255,255,0.5)] font-mono text-xs">{faucet.id}</span>
                <span className="text-[rgba(255,255,255,0.3)] text-xs">&rarr;</span>
                <span className="text-[rgba(255,255,255,0.5)] text-xs font-mono">{faucet.pair}</span>
              </div>
            ))}
          </div>
        </section>
      </div>

      <Footer />

      <ChartModal asset={selectedAsset} isOpen={isModalOpen} onClose={handleCloseModal} />
    </main>
  );
}
