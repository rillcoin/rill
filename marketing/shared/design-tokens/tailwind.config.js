/** @type {import('tailwindcss').Config} */
module.exports = {
  theme: {
    extend: {
      colors: {
        rill: {
          'dark-navy': '#0A1628',
          'deep-water': '#1A3A5C',
          'flowing-blue': '#3B82F6',
          'accent-orange': '#F97316',
          'light-gray': '#94A3B8',
        },
      },
      fontFamily: {
        headline: ['Instrument Serif', 'serif'],
        body: ['Inter', 'sans-serif'],
        code: ['JetBrains Mono', 'monospace'],
      },
      spacing: {
        '4xl': '96px',
      },
    },
  },
};
