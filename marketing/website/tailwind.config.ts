import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./src/pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/components/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        serif: ["var(--font-instrument-serif)", "Georgia", "serif"],
        sans: ["var(--font-inter)", "system-ui", "sans-serif"],
        mono: ["var(--font-jetbrains-mono)", "Menlo", "monospace"],
      },
      colors: {
        // Design token colors — sourced from shared/design-tokens/tokens.json
        void: "var(--void)",
        base: "var(--base)",
        raised: "var(--raised)",
        "blue-500": "var(--blue-500)",
        "blue-400": "var(--blue-400)",
        "blue-600": "var(--blue-600)",
        "cyan-400": "var(--cyan-400)",
        "orange-500": "var(--orange-500)",
        "orange-400": "var(--orange-400)",
        "text-primary": "var(--text-primary)",
        "text-secondary": "var(--text-secondary)",
        "text-muted": "var(--text-muted)",
        "text-dim": "var(--text-dim)",
        "text-faint": "var(--text-faint)",
      },
      backgroundImage: {
        "logo-gradient": "linear-gradient(135deg, #4A8AF4 0%, #22D3EE 100%)",
        "orange-gradient": "linear-gradient(135deg, #F97316 0%, #FB923C 100%)",
        "hero-bg":
          "linear-gradient(180deg, #020408 0%, #040B16 50%, #020408 100%)",
        "hero-glow":
          "radial-gradient(ellipse 65% 85% at 74% 44%, #0C2040 0%, transparent 100%)",
        "orb-primary":
          "radial-gradient(ellipse at 30% 30%, #1B58B0 0%, #0C2040 35%, #040B16 60%, #020408 85%)",
        "orb-secondary":
          "radial-gradient(ellipse at 60% 60%, rgba(34,211,238,0.125) 0%, transparent 50%)",
        "decay-glow":
          "radial-gradient(ellipse at 50% 50%, #0C2448 0%, transparent 70%)",
        "blue-cyan-gradient":
          "linear-gradient(180deg, #4A8AF4 0%, #22D3EE 100%)",
      },
      animation: {
        "pulse-slow": "pulse 4s cubic-bezier(0.4, 0, 0.6, 1) infinite",
      },
      screens: {
        "2xl": "1440px",
      },
    },
  },
  plugins: [],
};

export default config;
