import type { ReactNode } from 'react';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { CLASE_ACTIVO, CLASE_BORDE, CLASE_REPOSO, CLASE_TEXTO } from '../disenno';

function BotonPagina({
  etiqueta,
  icono,
  deshabilitado,
  alElegir,
}: {
  etiqueta: string;
  icono: ReactNode;
  deshabilitado: boolean;
  alElegir: () => void;
}) {
  return (
    <button
      type="button"
      onClick={alElegir}
      disabled={deshabilitado}
      aria-label={etiqueta}
      className={`flex h-10 shrink-0 cursor-pointer items-center gap-1 rounded-none border ${CLASE_BORDE} ${CLASE_REPOSO} px-3 ${CLASE_TEXTO} disabled:cursor-default disabled:opacity-30`}
    >
      {icono}
      {etiqueta}
    </button>
  );
}

export function Paginacion({
  pagina,
  totalPaginas,
  irA,
}: {
  pagina: number;
  totalPaginas: number;
  irA: (n: number) => void;
}) {
  if (totalPaginas <= 1) return null;
  return (
    <nav className="mt-5 flex w-full items-center justify-center gap-1" aria-label="Paginación">
      <BotonPagina
        etiqueta="Anterior"
        icono={<ChevronLeft className="h-4 w-4" />}
        deshabilitado={pagina <= 1}
        alElegir={() => irA(pagina - 1)}
      />
      {Array.from({ length: totalPaginas }, (_, n) => n + 1).map((n) => (
        <button
          key={n}
          type="button"
          onClick={() => irA(n)}
          aria-current={n === pagina ? 'page' : undefined}
          className={`h-10 w-10 shrink-0 cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_TEXTO} ${
            n === pagina ? CLASE_ACTIVO : CLASE_REPOSO
          }`}
        >
          {n}
        </button>
      ))}
      <BotonPagina
        etiqueta="Siguiente"
        icono={<ChevronRight className="h-4 w-4" />}
        deshabilitado={pagina >= totalPaginas}
        alElegir={() => irA(pagina + 1)}
      />
    </nav>
  );
}
