import type { Metadata } from "next";
import { Inter } from "next/font/google";
import Script from "next/script";
import "./globals.css";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Oracle Explorer | Pragma Miden",
  description: "Explore real-time oracle price feeds",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <Script id="wallet-fix" strategy="beforeInteractive">
          {`
            (function() {
              const descriptor = Object.getOwnPropertyDescriptor(window, 'ethereum');
              if (descriptor && !descriptor.configurable) {
                return;
              }
              Object.defineProperty(window, 'ethereum', {
                set: function(value) {
                  Object.defineProperty(window, 'ethereum', {
                    value: value,
                    writable: true,
                    configurable: true,
                    enumerable: true
                  });
                },
                configurable: true,
                enumerable: true
              });
            })();
          `}
        </Script>
        {children}
      </body>
    </html>
  );
}
