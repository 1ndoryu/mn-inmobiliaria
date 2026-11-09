import { useCallback, useEffect, useState } from 'react';
import { guardarConfigVersiones, leerConfigVersiones } from '../../data/inmuebles/versiones-api';

/* [30AA-1] Estado de la pestaña Versiones: valor guardado, borrador y aviso.
 * Leer y guardar comparten el mismo `mensaje`, por eso viven juntos. */
export function useConfigVersiones() {
  const [versiones, setVersiones] = useState<number | null>(null);
  const [borrador, setBorrador] = useState(2);
  const [guardando, setGuardando] = useState(false);
  const [mensaje, setMensaje] = useState<string | null>(null);

  useEffect(() => {
    let vivo = true;
    leerConfigVersiones()
      .then((r) => {
        if (!vivo) return;
        setVersiones(r.versiones);
        setBorrador(r.versiones);
      })
      .catch((e: unknown) => {
        if (vivo) setMensaje(e instanceof Error ? e.message : 'No se pudo leer la configuración.');
      });
    return () => {
      vivo = false;
    };
  }, []);

  const guardar = useCallback(async () => {
    setGuardando(true);
    setMensaje(null);
    try {
      const r = await guardarConfigVersiones(borrador);
      setVersiones(r.versiones);
      setBorrador(r.versiones);
      setMensaje('Guardado. Se aplica al abrir cada inmueble.');
    } catch (e) {
      setMensaje(e instanceof Error ? e.message : 'No se pudo guardar.');
    } finally {
      setGuardando(false);
    }
  }, [borrador]);

  /* Acotado en el propio cambio: el backend también recorta, pero el campo no
   * debe dejar escribir 99 mientras se teclea. */
  const cambiar = useCallback((valor: number) => setBorrador(Math.min(5, Math.max(1, valor || 1))), []);

  return { versiones, borrador, guardando, mensaje, guardar, cambiar };
}
