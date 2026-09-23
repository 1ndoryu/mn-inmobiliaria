import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { obtenerElementoPorId } from './platform/documento'

createRoot(obtenerElementoPorId('root')).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
