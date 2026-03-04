"use client";

const footerLinks = {
  Developers: [
    { label: "Documentation", href: "https://docs.pragma.build" },
    { label: "GitHub", href: "https://github.com/astraly-labs/pragma-miden" },
    { label: "Block Explorer", href: "https://testnet.midenscan.com" },
  ],
  Pragma: [
    { label: "Home", href: "https://pragma.build" },
    { label: "Explorer", href: "/" },
    { label: "Ecosystem", href: "https://pragma.build/ecosystem" },
  ],
  Community: [
    { label: "Twitter", href: "https://twitter.com/PragmaOracle" },
    { label: "Discord", href: "https://discord.gg/M9aRZtZHU7" },
    { label: "Telegram", href: "https://t.me/+Xri-uUMpWXI3ZmRk" },
  ],
};

export function Footer() {
  return (
    <footer className="border-t border-[rgba(255,255,255,0.06)] mt-24">
      <div className="max-w-6xl mx-auto px-6 lg:px-8 py-16">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-10">
          <div className="col-span-2 md:col-span-1">
            <div className="flex items-center gap-1.5 mb-3">
              <span className="text-white font-medium text-sm">Pragma</span>
              <span className="text-[rgba(255,255,255,0.3)] text-sm">/</span>
              <span className="text-[rgba(255,255,255,0.4)] text-sm">Miden</span>
            </div>
            <p className="text-[rgba(255,255,255,0.3)] text-sm leading-relaxed">
              Provable oracle feeds for Polygon Miden.
            </p>
          </div>
          {Object.entries(footerLinks).map(([category, links]) => (
            <div key={category}>
              <h4 className="text-white text-xs font-medium uppercase tracking-widest mb-4">{category}</h4>
              <ul className="space-y-2.5">
                {links.map((link) => (
                  <li key={link.label}>
                    <a
                      href={link.href}
                      target={link.href.startsWith("http") ? "_blank" : undefined}
                      rel={link.href.startsWith("http") ? "noopener noreferrer" : undefined}
                      className="text-[rgba(255,255,255,0.35)] hover:text-white transition-colors text-sm"
                    >
                      {link.label}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
        <div className="border-t border-[rgba(255,255,255,0.06)] mt-12 pt-8 flex items-center justify-between">
          <p className="text-[rgba(255,255,255,0.25)] text-xs">&copy; Pragma Labs &mdash; {new Date().getFullYear()}</p>
          <p className="text-[rgba(255,255,255,0.15)] text-xs font-mono">miden.pragma.build</p>
        </div>
      </div>
    </footer>
  );
}
