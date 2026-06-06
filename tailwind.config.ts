export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        surface: {
          base: '#0a0a0b',
          elevated: '#141416',
          hover: '#1c1c1f',
          active: '#222226',
          inset: '#0e0e10',
        },
        accent: {
          primary: '#3b82f6',
          secondary: '#8b5cf6',
        },
        border: {
          subtle: 'rgba(255,255,255,0.06)',
          strong: 'rgba(255,255,255,0.10)',
          accent: '#3b82f6',
        },
        text: {
          primary: '#f0f0f5',
          secondary: '#a0a0a8',
          muted: '#6e6e78',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
      },
      boxShadow: {
        panel: '0 1px 3px rgba(0,0,0,0.4)',
        elevated: '0 4px 12px rgba(0,0,0,0.5)',
      },
      borderRadius: {
        sm: '4px',
        md: '8px',
        lg: '12px',
      },
    },
  },
  plugins: [],
}
