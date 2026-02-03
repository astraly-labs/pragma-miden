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
    <header className="border-b border-border bg-surface">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between py-5">
          <div>
            <h1 className="text-2xl font-bold text-text-primary">
              Oracle Explorer
            </h1>
            <p className="text-text-muted mt-1 text-sm flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse"></span>
              Pragma Miden
            </p>
          </div>

          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-primary animate-pulse"></div>
            {lastUpdate && (
              <div className="text-xs text-text-muted font-mono">
                {formatTime(lastUpdate)}
              </div>
            )}
          </div>
        </div>
      </div>
    </header>
  );
}
