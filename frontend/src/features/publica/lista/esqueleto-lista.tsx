import {
  ALTO_CAJA,
  CLASE_RELLENO_SUAVE,
  MARGEN_IMAGEN_CAJA,
  POR_PAGINA,
  SOLO_ESCRITORIO_BLOQUE,
  SOLO_ESCRITORIO_FLEX,
  TAMANO_IMAGEN_CAJA,
} from '../disenno';

/* [249A-5] Esqueleto de la lista con las medidas de `CajaInmueble`
 * (`ALTO_CAJA` fija la altura: el contenido nunca mueve la fila). PSI
 * escritorio medía CLS 0.484 porque `Cargando…` (una línea) saltaba a la
 * lista completa al llegar la API (~515 ms). Con `POR_PAGINA` filas el
 * alto cargando == alto cargado y el salto desaparece. `animate-pulse`
 * solo toca opacidad: no genera CLS. */
function FilaEsqueleto() {
  return (
    <div aria-hidden className={`flex ${ALTO_CAJA} items-center`}>
      <div className={`${MARGEN_IMAGEN_CAJA} ${TAMANO_IMAGEN_CAJA} shrink-0 ${CLASE_RELLENO_SUAVE}`} />
      <div className="flex min-w-0 flex-1 items-center self-stretch px-4">
        <div className={`h-4 w-2/3 ${CLASE_RELLENO_SUAVE} animate-pulse`} />
      </div>
      <div className={`min-w-0 flex-1 items-stretch justify-center self-stretch ${SOLO_ESCRITORIO_FLEX}`}>
        <div className={`h-4 w-16 self-center ${CLASE_RELLENO_SUAVE} animate-pulse`} />
      </div>
      <div className={`m-0 w-28 shrink-0 px-4 ${SOLO_ESCRITORIO_BLOQUE}`}>
        <div className={`h-4 w-full ${CLASE_RELLENO_SUAVE} animate-pulse`} />
      </div>
    </div>
  );
}

export function EsqueletoLista({ filas = POR_PAGINA }: { filas?: number }) {
  return (
    <>
      {Array.from({ length: filas }, (_, n) => (
        <FilaEsqueleto key={n} />
      ))}
    </>
  );
}
