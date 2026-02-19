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
        sans:  ["var(--font-inter)", "system-ui", "sans-serif"],
        mono:  ["var(--font-jetbrains-mono)", "monospace"],
      },
      colors: {
        void:           "var(--void)",
        base:           "var(--base)",
        raised:         "var(--raised)",
        "blue-500":     "var(--blue-500)",
        "blue-400":     "var(--blue-400)",
        "blue-600":     "var(--blue-600)",
        "cyan-400":     "var(--cyan-400)",
        "orange-500":   "var(--orange-500)",
        "orange-400":   "var(--orange-400)",
        "text-primary": "var(--text-primary)",
        "text-secondary":"var(--text-secondary)",
        "text-muted":   "var(--text-muted)",
        "text-dim":     "var(--text-dim)",
        "text-faint":   "var(--text-faint)",
      },
      screens: {
        "2xl": "1440px",
      },
    },
  },
  plugins: [],
};

export default config;
