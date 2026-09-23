// Config del chat (169A-5): lee el mapa del backend, edita en local y
// guarda solo lo cambiado. `ai_enabled_global` es el kill-switch de la IA.

import { useCallback, useEffect, useRef, useState } from 'react';
import { guardarConfig, leerConfig, type ClaveConfig, type MapaConfig } from '../../data/chat/cliente-admin';
import { ErrorApi } from '../../data/inmuebles/api';

export function useConfigChat() {
  const [valores, setValores] = useState<MapaConfig | null>(null);
  const [cargando, setCargando] = useState(true);
  const [guardando, setGuardando] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [aviso, setAviso] = useState<string | null>(null);
  /* Foto al cargar (y tras guardar): el guardado envía solo el diff. */
  const base = useRef<MapaConfig | null>(null);

  const recargar = useCallback(async () => {
    try {
      const mapa = await leerConfig();
      base.current = mapa;
      setValores(mapa);
      setError(null);
    } catch (e) {
      setError(e instanceof ErrorApi ? e.message : 'No se pudo leer la configuración.');
    } finally {
      setCargando(false);
    }
  }, []);

  useEffect(() => {
    void recargar();
  }, [recargar]);

  function poner(clave: ClaveConfig, valor: string): void {
    setValores((v) => (v ? { ...v, [clave]: valor } : v));
    setAviso(null);
  }

  async function guardar(): Promise<void> {
    if (!valores || !base.current) return;
    const cambios: Partial<Record<ClaveConfig, string>> = {};
    for (const [clave, valor] of Object.entries(valores)) {
      if (valor !== null && valor !== base.current[clave as ClaveConfig]) {
        cambios[clave as ClaveConfig] = valor;
      }
    }
    if (Object.keys(cambios).length === 0) {
      setAviso('Sin cambios que guardar.');
      return;
    }
    setGuardando(true);
    try {
      await guardarConfig(cambios);
      base.current = valores;
      setAviso('Configuración guardada.');
      setError(null);
    } catch (e) {
      setError(e instanceof ErrorApi ? e.message : 'No se pudo guardar.');
    } finally {
      setGuardando(false);
    }
  }

  return { valores, cargando, guardando, error, aviso, poner, guardar, recargar };
}
