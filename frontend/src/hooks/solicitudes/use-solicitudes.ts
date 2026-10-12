/* [169A-2] Estado del panel de solicitudes: listado paginado por estado y
 * cambio de revisión. Tras cada cambio se vuelve a pedir la página para que
 * la fila salga del filtro si ya no le corresponde y el total quede al día. */
import { useEffect, useState } from 'react';
import { cambiarEstadoSolicitud, listarSolicitudes } from '../../data/solicitudes/api';
import type { EstadoSolicitud, PaginaSolicitudes, Solicitud } from '../../domain/solicitud';

export const POR_PAGINA = 20;

export type FiltroEstado = 'todos' | EstadoSolicitud;

export function useSolicitudes() {
  const [filtro, setFiltro] = useState<FiltroEstado>('pendiente');
  const [pagina, setPagina] = useState(1);
  const [datos, setDatos] = useState<PaginaSolicitudes | null>(null);
  const [cargando, setCargando] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [guardandoId, setGuardandoId] = useState<string | null>(null);
  /* Contador para volver a pedir la página tras un cambio de estado. */
  const [version, setVersion] = useState(0);

  useEffect(() => {
    let vigente = true;
    listarSolicitudes({ estado: filtro === 'todos' ? undefined : filtro, page: pagina, perPage: POR_PAGINA })
      .then((p) => {
        if (!vigente) return;
        setDatos(p);
        setError(null);
      })
      .catch((e: unknown) => {
        if (vigente) setError(e instanceof Error ? e.message : 'No se pudieron cargar las solicitudes.');
      })
      .finally(() => {
        if (vigente) setCargando(false);
      });
    return () => {
      vigente = false;
    };
  }, [filtro, pagina, version]);

  function cambiarFiltro(valor: FiltroEstado) {
    setFiltro(valor);
    setPagina(1);
    setCargando(true);
  }

  function irAPagina(p: number) {
    setPagina(p);
    setCargando(true);
  }

  /* El error del PATCH se muestra en la barra; nunca se traga. */
  async function cambiarEstado(s: Solicitud, estado: EstadoSolicitud) {
    setGuardandoId(s.id);
    setError(null);
    try {
      await cambiarEstadoSolicitud(s.id, estado);
      setCargando(true);
      setVersion((v) => v + 1);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'No se pudo cambiar el estado.');
    } finally {
      setGuardandoId(null);
    }
  }

  const total = datos?.total ?? 0;
  const numPaginas = Math.max(1, Math.ceil(total / POR_PAGINA));

  return { filtro, cambiarFiltro, pagina, irAPagina, datos, cargando, error, guardandoId, cambiarEstado, total, numPaginas };
}
