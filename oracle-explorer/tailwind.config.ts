import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./components/**/*.{js,ts,jsx,tsx,mdx}",
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        background: "#0a0a0a",
        surface: "#141414",
        "surface-elevated": "#1a1a1a",
        border: "#2a2a2a",
        "border-hover": "#3a3a3a",
        primary: "#00ff88",
        "primary-hover": "#00cc6a",
        secondary: "#3b82f6",
        accent: "#8b5cf6",
        danger: "#ef4444",
        "text-primary": "#ffffff",
        "text-secondary": "#a0a0a0",
        "text-muted": "#6b6b6b",
      },
      animation: {
        "pulse-slow": "pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite",
        "fade-in": "fadeIn 0.3s ease-in-out",
      },
      keyframes: {
        fadeIn: {
          "0%": { opacity: "0", transform: "translateY(10px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
      },
    },
  },
  plugins: [],
};
export default config;
