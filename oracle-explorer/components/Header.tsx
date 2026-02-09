"use client";

interface HeaderProps {
  lastUpdate: Date | null;
}

export function Header({ lastUpdate }: HeaderProps) {
  const formatTime = (date: Date) => {
    return date.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  };

  return (
    <header className="border-b border-border bg-gradient-to-r from-surface via-surface-elevated to-surface backdrop-blur-xl sticky top-0 z-50 shadow-2xl shadow-black/30">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between py-6">
          <div className="flex items-center gap-4">
            <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-surface-elevated to-background border border-border shadow-lg flex items-center justify-center">
              <span className="text-primary font-bold text-lg">P</span>
            </div>
            <div>
              <h1 className="text-3xl font-bold text-text-primary tracking-tight bg-gradient-to-r from-white to-text-secondary bg-clip-text text-transparent">
                Oracle Explorer
              </h1>
              <p className="text-text-secondary mt-1 text-sm flex items-center gap-2 font-medium">
                <span className="relative flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-primary shadow-lg shadow-primary/50"></span>
                </span>
                Pragma Miden
              </p>
            </div>
          </div>

          <div className="flex items-center gap-3 px-5 py-3 rounded-xl bg-black/30 backdrop-blur-sm border border-white/10 shadow-lg">
            <div className="relative flex h-2.5 w-2.5">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-primary shadow-lg shadow-primary/50"></span>
            </div>
            {lastUpdate && (
              <div className="text-xs text-text-secondary font-mono tracking-wider">
                {formatTime(lastUpdate)}
              </div>
            )}
          </div>
        </div>
      </div>
    </header>
  );
}
