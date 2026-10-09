import type { Config } from 'tailwindcss';

export default {
  content: ['./src/**/*.{html,js,svelte,ts}', './index.html'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Carbon Blue 60 系（品牌主色）
        primary: {
          50: '#edf5ff',
          100: '#d0e2ff',
          200: '#a6c8ff',
          300: '#78a9ff',
          400: '#4589ff',
          500: '#0f62fe',
          600: '#0353e9',
          700: '#0043ce',
          800: '#002d9c',
          900: '#001d6c',
        },
        // Carbon Gray（中性色，业务皮肤跟随 Carbon 配色）
        slate: {
          50: '#f4f4f4',
          100: '#e0e0e0',
          200: '#c6c6c6',
          300: '#a8a8a8',
          400: '#8d8d8d',
          500: '#6f6f6f',
          600: '#525252',
          700: '#393939',
          800: '#262626',
          900: '#161616',
        },
        // 语义色：Carbon 绿/黄/红
        emerald: {
          300: '#a7f0ba',
          400: '#6fdc8c',
          500: '#42be65',
          600: '#24a148',
          700: '#198038',
          900: '#0e6027',
        },
        amber: {
          50: '#fcf4d6',
          100: '#f8e6ac',
          300: '#fdd13a',
          400: '#f1c21b',
          500: '#d2a106',
          600: '#b28600',
          700: '#8e6a00',
          900: '#5e3700',
        },
        rose: {
          300: '#ffd7d9',
          400: '#ff8389',
          500: '#fa4d56',
          600: '#da1e28',
          700: '#a2191f',
          900: '#750e13',
        },
      },
      fontFamily: {
        sans: [
          'IBM Plex Sans',
          'IBM Plex Sans Arabic',
          '-apple-system',
          'BlinkMacSystemFont',
          'SF Pro Text',
          'PingFang SC',
          'Hiragino Sans GB',
          'Segoe UI Variable Text',
          'Microsoft YaHei',
          'sans-serif',
        ],
        mono: ['Fira Code', 'monospace'],
      },
    },
  },
  plugins: [],
} satisfies Config;
