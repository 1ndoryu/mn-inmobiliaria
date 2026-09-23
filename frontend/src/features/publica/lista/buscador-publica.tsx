import { Search, SlidersHorizontal } from 'lucide-react';
import { CLASE_ACENTO, CLASE_ACTIVO, CLASE_BORDE, CLASE_TEXTO } from '../disenno';

/* Buscador pegado sobre los filtros: el borde vive en la caja,
 * el input va sin bordes. En escritorio la caja no lleva borde inferior
 * porque la fila de tipos la continúa; en móvil y tableta esa fila se
 * oculta y la caja conserva su borde inferior. El icono abre el modal de filtros
 * avanzados y se rellena cuando hay filtros aplicados. */
export function BuscadorPublica({
  valor,
  alCambiar,
  alAbrirFiltros,
  filtrosActivos,
}: {
  valor: string;
  alCambiar: (valor: string) => void;
  alAbrirFiltros: () => void;
  filtrosActivos: boolean;
}) {
  return (
    <div className={`flex w-full items-center gap-2 border lg:border-b-0 ${CLASE_BORDE} px-4 py-3`}>
      <Search className="h-4 w-4 shrink-0" aria-hidden />
      <input
        type="search"
        placeholder="Buscar por título o ubicación…"
        aria-label="Buscar inmuebles"
        value={valor}
        onChange={(e) => alCambiar(e.target.value)}
        className={`min-w-0 flex-1 border-0 bg-transparent p-0 ${CLASE_TEXTO} outline-none placeholder:text-black/40`}
      />
      <button
        type="button"
        aria-label="Filtros avanzados"
        aria-pressed={filtrosActivos}
        onClick={alAbrirFiltros}
        className={`flex shrink-0 cursor-pointer items-center rounded-none border p-1 ${
          filtrosActivos ? `${CLASE_BORDE} ${CLASE_ACTIVO}` : 'border-transparent bg-transparent'
        }`}
      >
        <SlidersHorizontal className={`h-4 w-4 ${filtrosActivos ? CLASE_TEXTO : CLASE_ACENTO}`} />
      </button>
    </div>
  );
}
