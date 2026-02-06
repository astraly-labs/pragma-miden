"use client";

export function Footer() {
  return (
    <footer className="border-t border-border bg-gradient-to-r from-surface via-surface-elevated to-surface mt-16">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="flex flex-col md:flex-row items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-surface-elevated to-background border border-border flex items-center justify-center">
              <span className="text-text-primary font-bold text-xs">PM</span>
            </div>
            <div>
              <p className="text-text-primary font-semibold text-sm">Pragma Miden</p>
              <p className="text-text-muted text-xs">Oracle Explorer</p>
            </div>
          </div>
          
          <div className="flex items-center gap-6">
            <a 
              href="https://github.com/pragma-oracle" 
              target="_blank" 
              rel="noopener noreferrer"
              className="text-text-secondary hover:text-text-primary transition-colors text-sm font-medium"
            >
              GitHub
            </a>
            <a 
              href="https://docs.pragma.build" 
              target="_blank" 
              rel="noopener noreferrer"
              className="text-text-secondary hover:text-text-primary transition-colors text-sm font-medium"
            >
              Documentation
            </a>
            <a 
              href="https://pragma.build" 
              target="_blank" 
              rel="noopener noreferrer"
              className="text-text-secondary hover:text-text-primary transition-colors text-sm font-medium"
            >
              Website
            </a>
          </div>
          
          <p className="text-text-muted text-xs">
            Powered by Miden Rollup
          </p>
        </div>
      </div>
    </footer>
  );
}
