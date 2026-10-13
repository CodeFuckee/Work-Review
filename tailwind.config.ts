import type { Config } from 'tailwindcss';

export default {
  content: ['./src/**/*.{html,js,svelte,ts}', './index.html'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Material Blue 系（品牌主色，与 md-theme.css 的 --md-theme-primary=#1976d2 同源）
        primary: {
          50: '#e3f2fd',
          100: '#bbdefb',
          200: '#90caf9',
          300: '#64b5f6',
          400: '#42a5f5',
          500: '#2196f3',
          600: '#1e88e5',
          700: '#1976d2',
          800: '#1565c0',
          900: '#0d47a1',
        },
        // Material Grey（中性色，业务皮肤跟随 Material 配色）
        slate: {
          50: '#fafafa',
          100: '#f5f5f5',
          200: '#eeeeee',
          300: '#e0e0e0',
          400: '#bdbdbd',
          500: '#9e9e9e',
          600: '#757575',
          700: '#616161',
          800: '#424242',
          900: '#212121',
        },
        // 语义色：Material Green/Amber/Red
        emerald: {
          300: '#a5d6a7',
          400: '#81c784',
          500: '#66bb6a',
          600: '#43a047',
          700: '#388e3c',
          900: '#1b5e20',
        },
        amber: {
          50: '#fff8e1',
          100: '#ffecb3',
          300: '#ffd54f',
          400: '#ffca28',
          500: '#ffb300',
          600: '#ffa000',
          700: '#ff8f00',
          900: '#e65100',
        },
        rose: {
          300: '#ef9a9a',
          400: '#e57373',
          500: '#b00020',
          600: '#b00020',
          700: '#8e0018',
          900: '#5b0010',
        },
      },
      fontFamily: {
        sans: [
          'Roboto',
          'Noto Sans SC',
          'Noto Sans Arabic',
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
