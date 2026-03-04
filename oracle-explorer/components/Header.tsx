"use client";

import { useState } from "react";
import { PragmaTransitionLink } from "@/components/TransitionOverlay";

interface HeaderProps {
  lastUpdate: Date | null;
}

export function Header({ lastUpdate }: HeaderProps) {
  const [mobileOpen, setMobileOpen] = useState(false);

  const formatTime = (date: Date) =>
    date.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit", second: "2-digit" });

  return (
    <header className="border-b border-[rgba(255,255,255,0.06)] bg-[#080808]/95 backdrop-blur-md sticky top-0 z-50">
      <div className="max-w-6xl mx-auto px-6 lg:px-8">
        <div className="flex items-center justify-between h-14">
          <a href="https://pragma.build" className="flex items-center gap-2">
            <span className="text-white font-medium text-sm tracking-tight">Pragma</span>
            <span className="text-[rgba(255,255,255,0.3)] text-sm">/</span>
            <span className="text-[rgba(255,255,255,0.5)] text-sm font-normal">Miden</span>
          </a>

          <nav className="hidden md:flex items-center gap-6">
            {[
              { label: "Explorer", href: "#prices" },
              { label: "Docs", href: "https://docs.pragma.build", external: true },
              { label: "GitHub", href: "https://github.com/astraly-labs/pragma-miden", external: true },
            ].map((link) => (
              <a
                key={link.label}
                href={link.href}
                target={link.external ? "_blank" : undefined}
                rel={link.external ? "noopener noreferrer" : undefined}
                className="text-[rgba(255,255,255,0.45)] hover:text-white transition-colors text-sm"
              >
                {link.label}
              </a>
            ))}
          </nav>

          <div className="flex items-center gap-4">
            {lastUpdate && (
              <div className="hidden sm:flex items-center gap-1.5 text-xs text-[rgba(255,255,255,0.3)] font-mono">
                <span className="relative flex h-1.5 w-1.5">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-white opacity-30"></span>
                  <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-white opacity-60"></span>
                </span>
                {formatTime(lastUpdate)}
              </div>
            )}
            <a
              href="https://docs.pragma.build"
              target="_blank"
              rel="noopener noreferrer"
              className="btn-primary text-xs px-4 py-2"
            >
              Start Building
            </a>
            <button
              onClick={() => setMobileOpen(!mobileOpen)}
              className="md:hidden p-1.5 text-[rgba(255,255,255,0.45)] hover:text-white transition-colors"
              aria-label="Toggle menu"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                {mobileOpen ? (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M6 18L18 6M6 6l12 12" />
                ) : (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 6h16M4 12h16M4 18h16" />
                )}
              </svg>
            </button>
          </div>
        </div>
      </div>

      {mobileOpen && (
        <div className="md:hidden border-t border-[rgba(255,255,255,0.06)] px-6 py-4 space-y-3">
          {lastUpdate && (
            <div className="flex items-center gap-1.5 text-xs text-[rgba(255,255,255,0.3)] font-mono pb-2">
              <span className="relative flex h-1.5 w-1.5">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-white opacity-30"></span>
                <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-white opacity-60"></span>
              </span>
              {formatTime(lastUpdate)}
            </div>
          )}
          <a href="#prices" onClick={() => setMobileOpen(false)} className="block text-sm text-[rgba(255,255,255,0.45)] hover:text-white py-1">Explorer</a>
          <a href="https://docs.pragma.build" target="_blank" rel="noopener noreferrer" className="block text-sm text-[rgba(255,255,255,0.45)] hover:text-white py-1">Docs</a>
          <a href="https://github.com/astraly-labs/pragma-miden" target="_blank" rel="noopener noreferrer" className="block text-sm text-[rgba(255,255,255,0.45)] hover:text-white py-1">GitHub</a>
          <a href="https://docs.pragma.build" target="_blank" rel="noopener noreferrer" className="btn-primary text-xs px-4 py-2 mt-2">Start Building</a>
        </div>
      )}
    </header>
  );
}
