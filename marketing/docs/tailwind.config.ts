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
        void: "var(--void)",
        base: "var(--base)",
        raised: "var(--raised)",
        surface: "var(--surface)",
        overlay: "var(--overlay)",
        "blue-300": "var(--blue-300)",
        "blue-400": "var(--blue-400)",
        "blue-500": "var(--blue-500)",
        "cyan-300": "var(--cyan-300)",
        "cyan-400": "var(--cyan-400)",
        "cyan-500": "var(--cyan-500)",
        "orange-400": "var(--orange-400)",
        "orange-500": "var(--orange-500)",
        "text-primary": "var(--text-primary)",
        "text-secondary": "var(--text-secondary)",
        "text-muted": "var(--text-muted)",
        "text-dim": "var(--text-dim)",
      },
    },
  },
  plugins: [],
};

export default config;
