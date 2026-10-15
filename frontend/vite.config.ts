import path from 'node:path';
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { defineConfig } from 'vite'

// Dev público por Cloudflare Tunnel: solo si botonRemoto define MN_DEV_PUBLIC_HOST.
// Sin la variable, la configuración es la de siempre (localhost).
const publicHost = process.env.MN_DEV_PUBLIC_HOST

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { '@': path.resolve(import.meta.dirname, 'src') },
  },
  server: {
    port: 5199,
    host: '127.0.0.1',
    strictPort: true,
    // Vite 8 rechaza el Host ajeno (anti DNS rebinding): el hostname público debe estar aquí.
    // HMR por wss en 443: el navegador habla con el borde de Cloudflare, no con 5199.
    ...(publicHost && {
      allowedHosts: [publicHost],
      hmr: { protocol: 'wss', host: publicHost, clientPort: 443 },
    }),
    // Mismo-origen hacia GloryAPI: evita CORS sin tocar gloryapi.
    proxy: {
      '/glory': {
        target: 'http://127.0.0.1:3101',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/glory/, ''),
      },
      // Mini-backend local de mejora (server/mejora.mjs en :3122).
      '/api': {
        target: 'http://127.0.0.1:3122',
        changeOrigin: true,
      },
      // API Rust en dev público (VITE_API_URL=https://<host>/rust): mismo origen que el front,
      // sin CORS ni cookies de Access entre subdominios (fetch cross-origin no envía cookies).
      '/rust': {
        target: 'http://127.0.0.1:3110',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/rust/, ''),
      },
      /* [299A-3] Media del backend Rust (:3110 [03AA-1]: el 3000 lo ocupa glory-pulse): el hilo muestra
       * `/uploads/...` relativo (en prod es mismo origen); en dev el
       * proxy lo lleva al backend para que `<img>`/`<audio>` carguen. */
      '/uploads': {
        target: 'http://127.0.0.1:3110',
        changeOrigin: true,
      },
    },
  },
  preview: {
    port: 5199,
    host: '127.0.0.1',
  },
})
