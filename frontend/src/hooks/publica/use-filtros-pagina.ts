/* Filtro por tipo + busqueda + pagina de la pagina publica. Extraido del
 * componente para cumplir la regla de maximo 3 useState por componente:
 * la pagina solo conserva `chatAbierto`. */
import { useState } from 'react';
import type { TipoInmueble } from '../../domain/inmueble';

export function useFiltrosPagina() {
  const [filtro, setFiltro] = useState<TipoInmueble | null>(null);
  const [pagina, setPagina] = useState(1);
  const [busqueda, setBusqueda] = useState('');

  function elegirFiltro(t: TipoInmueble | null) {
    setFiltro(t);
    setPagina(1);
  }

  function alBuscar(valor: string) {
    setBusqueda(valor);
    setPagina(1);
  }

  return { filtro, pagina, setPagina, busqueda, elegirFiltro, alBuscar };
}
