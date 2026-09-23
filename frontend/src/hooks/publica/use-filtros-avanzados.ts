import { useCallback, useState } from 'react';
import {
  FILTROS_AVANZADOS_VACIOS,
  hayFiltrosAvanzados,
  type FiltrosAvanzados,
} from '../../features/publica/busqueda';

/* Modal "Filtros avanzados": el borrador se conserva al cerrar sin aplicar
 * (igual que el modal publicar) y solo `aplicados` filtra la lista. */
export function useFiltrosAvanzados() {
  const [abierto, setAbierto] = useState(false);
  const [borrador, setBorrador] = useState<FiltrosAvanzados>(FILTROS_AVANZADOS_VACIOS);
  const [aplicados, setAplicados] = useState<FiltrosAvanzados>(FILTROS_AVANZADOS_VACIOS);

  /* Al reabrir el borrador se conserva (igual que el modal publicar):
   * lo no aplicado no se pierde al cerrar con overlay o Escape. */
  const abrir = useCallback(() => setAbierto(true), []);

  const cerrar = useCallback(() => setAbierto(false), []);

  const actualizar = useCallback((campo: keyof FiltrosAvanzados, valor: string) => {
    setBorrador((b) => ({ ...b, [campo]: valor }));
  }, []);

  const aplicar = useCallback(() => {
    setAplicados(borrador);
    setAbierto(false);
  }, [borrador]);

  const limpiar = useCallback(() => {
    setBorrador(FILTROS_AVANZADOS_VACIOS);
    setAplicados(FILTROS_AVANZADOS_VACIOS);
  }, []);

  return {
    abierto,
    abrir,
    cerrar,
    borrador,
    actualizar,
    aplicar,
    limpiar,
    aplicados,
    hayActivos: hayFiltrosAvanzados(aplicados),
  };
}
