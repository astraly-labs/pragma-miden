import type { Config } from "tailwindcss";
import plugin from "tailwindcss/plugin";

const config: Config = {
  content: [
    "./pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./components/**/*.{js,ts,jsx,tsx,mdx}",
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        background: "#080808",
        surface: "#111111",
        "surface-elevated": "#161616",
        border: "rgba(255,255,255,0.08)",
        "border-hover": "rgba(255,255,255,0.16)",
        "text-primary": "#ffffff",
        "text-secondary": "#888888",
        "text-muted": "#444444",
        accent: "#15FF81",
        danger: "#ef4444",
        "code-color": "#a8b3cf",
        // Keep these for ChartModal compatibility
        primary: "#15FF81",
        "primary-hover": "#10cc66",
        darkGreen: "#080808",
        lightGreen: "#ffffff",
        mint: "#15FF81",
        lightBlur: "rgba(255,255,255,0.08)",
        redDown: "#ef4444",
      },
      fontFamily: {
        sans: ["IBM Plex Sans", "ui-sans-serif", "system-ui", "sans-serif"],
        mono: ["IBM Plex Mono", "ui-monospace", "SFMono-Regular", "monospace"],
      },
      animation: {
        "fade-in": "fadeIn 0.4s ease-out forwards",
        "bounce-subtle": "bounceSubtle 0.6s ease-in-out",
      },
      keyframes: {
        fadeIn: {
          "0%": { opacity: "0", transform: "translateY(8px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        bounceSubtle: {
          "0%, 100%": { transform: "translateY(0)" },
          "50%": { transform: "translateY(-4px)" },
        },
      },
    },
  },
  plugins: [
    plugin(function ({ addComponents }) {
      addComponents({
        ".h1": {
          fontSize: "36px",
          fontWeight: "300",
          lineHeight: "44px",
          letterSpacing: "-0.72px",
          "@media (min-width: 768px)": {
            fontSize: "60px",
            lineHeight: "70px",
            letterSpacing: "-1.2px",
          },
        },
        ".h2": {
          fontSize: "28px",
          fontWeight: "300",
          lineHeight: "36px",
          letterSpacing: "-0.56px",
          "@media (min-width: 768px)": {
            fontSize: "42px",
            lineHeight: "52px",
            letterSpacing: "-0.84px",
          },
        },
      });
    }),
  ],
};
export default config;
