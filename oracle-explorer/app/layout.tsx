import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Pragma Miden | Oracle Price Feeds",
  description: "Real-time provable oracle price feeds for Polygon Miden. Live BTC, ETH, SOL and more.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        {children}
      </body>
    </html>
  );
}
