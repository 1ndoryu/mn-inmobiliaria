import { useCallback, useEffect, useState } from 'react';
import { leerEstado, leerEventos, type EstadoCola, type EventoCola } from '../../data/mejora/cliente-mejora';

export interface Diagnostico {
  /** null = aún sin respuesta; `vivo` dice si el backend contesta. */
  estado: EstadoCola | null;
  eventos: EventoCola[];
  vivo: boolean;
  /** Recarga snapshot + eventos ahora mismo (tras probar/reiniciar). */
  refrescar: () => void;
}

/* Sondea el backend mientras la vista Imágenes está montada: snapshot cada
 * 10s y eventos cada 30s. Intervalos largos a propósito: es diagnóstico,
 * no seguimiento de trabajos (eso lo hace useColaMejora). */
export function useDiagnostico(): Diagnostico {
  const [estado, setEstado] = useState<EstadoCola | null>(null);
  const [eventos, setEventos] = useState<EventoCola[]>([]);
  const [vivo, setVivo] = useState(false);

  const cargarEstado = useCallback(async () => {
    const e = await leerEstado();
    setVivo(e !== null);
    if (e) setEstado(e);
  }, []);
  const cargarEventos = useCallback(async () => {
    const ev = await leerEventos(60);
    setEventos(ev);
  }, []);
  const refrescar = useCallback(() => {
    void cargarEstado();
    void cargarEventos();
  }, [cargarEstado, cargarEventos]);

  useEffect(() => {
    let cancelado = false;
    void (async () => {
      const e = await leerEstado();
      if (cancelado) return;
      setVivo(e !== null);
      if (e) setEstado(e);
      const ev = await leerEventos(60);
      if (!cancelado) setEventos(ev);
    })();
    const t1 = setInterval(() => void cargarEstado(), 10_000);
    const t2 = setInterval(() => void cargarEventos(), 30_000);
    return () => {
      cancelado = true;
      clearInterval(t1);
      clearInterval(t2);
    };
  }, [cargarEstado, cargarEventos]);

  return { estado, eventos, vivo, refrescar };
}
