/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./src/renderer/index.html",
    "./src/renderer/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        ping: {
          bg: '#0a0a0a',
          surface: '#111111',
          elevated: '#1a1a1a',
          border: '#2a2a2a',
          text: '#eaeaea',
          muted: '#888888',
          accent: '#4a9f6e',
          'accent-dim': '#3a7f5e',
          danger: '#e55050',
        }
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'Monaco', 'monospace'],
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
      animation: {
        'fade-in': 'fadeIn 150ms ease-out',
        'fade-out': 'fadeOut 150ms ease-in',
        'scale-in': 'scaleIn 200ms ease-out',
        'slide-up': 'slideUp 200ms ease-out',
        'pulse-soft': 'pulseSoft 2s ease-in-out infinite',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        fadeOut: {
          '0%': { opacity: '1' },
          '100%': { opacity: '0' },
        },
        scaleIn: {
          '0%': { opacity: '0', transform: 'scale(0.95)' },
          '100%': { opacity: '1', transform: 'scale(1)' },
        },
        slideUp: {
          '0%': { opacity: '0', transform: 'translateY(10px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        pulseSoft: {
          '0%, 100%': { opacity: '1' },
          '50%': { opacity: '0.7' },
        },
      },
      boxShadow: {
        'glow': '0 0 20px rgba(74, 159, 110, 0.15)',
        'soft': '0 4px 20px rgba(0, 0, 0, 0.5)',
      },
      backdropBlur: {
        'xs': '2px',
      },
    },
  },
  plugins: [],
}
