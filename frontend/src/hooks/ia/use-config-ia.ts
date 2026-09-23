import { useCallback, useEffect, useRef, useState } from 'react';
import { guardarConfigIA, leerEstadoIA, probarProveedor } from '@/data/ia/cliente-ia';
import type { EstadoIA, ProveedorIA } from '@/domain/ia';

/* Estado del centro de IA para Configuración (1 useState): carga el estado,
 * guarda activo+habilitados y prueba cada proveedor (el backend registra el
 * diagnóstico `ia_check_*`, así que tras probar se recarga). */

interface ModeloIA {
  estado: EstadoIA | null;
  cargando: boolean;
  guardando: boolean;
  probando: ProveedorIA | null;
  error: string | null;
}

const INICIAL: ModeloIA = { estado: null, cargando: true, guardando: false, probando: null, error: null };

export function useConfigIA(activo: boolean) {
  const [modelo, setModelo] = useState<ModeloIA>(INICIAL);
  const abortar = useRef<AbortController | null>(null);

  const recargar = useCallback(async (signal?: AbortSignal) => {
    try {
      const estado = await leerEstadoIA(signal);
      setModelo((m) => ({ ...m, estado, cargando: false, error: null }));
    } catch (e) {
      setModelo((m) => ({
        ...m,
        cargando: false,
        error: e instanceof Error ? e.message : 'No se pudo leer el estado de la IA.',
      }));
    }
  }, []);

  useEffect(() => {
    if (!activo) return;
    abortar.current?.abort();
    const ctrl = new AbortController();
    abortar.current = ctrl;
    setModelo(INICIAL);
    void recargar(ctrl.signal);
    return () => ctrl.abort();
  }, [activo, recargar]);

  useEffect(() => () => abortar.current?.abort(), []);

  const guardar = useCallback(
    async (proveedor: ProveedorIA, glory: boolean, open: boolean) => {
      setModelo((m) => ({ ...m, guardando: true, error: null }));
      try {
        await guardarConfigIA(proveedor, glory, open);
        await recargar();
      } catch (e) {
        setModelo((m) => ({ ...m, error: e instanceof Error ? e.message : 'No se pudo guardar.' }));
      }
      setModelo((m) => ({ ...m, guardando: false }));
    },
    [recargar],
  );

  const probar = useCallback(
    async (id: ProveedorIA) => {
      setModelo((m) => ({ ...m, probando: id, error: null }));
      await probarProveedor(id);
      /* El backend guarda el diagnóstico (`ia_check_*`) gane o pierda:
       * recargar lo muestra en la tarjeta. */
      await recargar();
      setModelo((m) => ({ ...m, probando: null }));
    },
    [recargar],
  );

  return { ...modelo, recargar, guardar, probar };
}
