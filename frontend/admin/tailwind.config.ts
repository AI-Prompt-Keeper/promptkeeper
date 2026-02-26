import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./src/pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/components/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        lavender: {
          DEFAULT: "#B19CD9",
          light: "#D4C5E8",
          dark: "#8B7BA8",
        },
        cream: {
          DEFAULT: "#FFF8E7",
          dark: "#F5EED6",
        },
        midnight: {
          DEFAULT: "#1A1F3A",
          light: "#2A3450",
        },
      },
      fontFamily: {
        sans: ["var(--font-geist-sans)", "system-ui", "sans-serif"],
        mono: ["var(--font-geist-mono)", "ui-monospace", "monospace"],
      },
    },
  },
  plugins: [],
};
export default config;
