import type { ReactNode } from 'react';
import { ArrowRight, Bath, BedDouble, CarFront, Ruler } from 'lucide-react';
import { portadaDe, miniaturaDe, type InmueblePublico } from '../../../domain/inmueble';
import {
  ALTO_CAJA,
  ANCHO_PRECIO,
  ANCHO_SPEC,
  ANCHO_SPEC_LARGO,
  CLASE_ACENTO,
  CLASE_HOVER_CAJA,
  CLASE_RELLENO_SUAVE,
  CLASE_TEXTO,
  MARGEN_IMAGEN_CAJA,
  SOLO_ESCRITORIO_BLOQUE,
  SOLO_ESCRITORIO_FLEX,
  TAMANO_IMAGEN_CAJA,
} from '../disenno';
import { formatearPrecio } from '../formato';

function CeldaSpec({ icono, valor, etiqueta, ancho = ANCHO_SPEC }: { icono: ReactNode; valor: string | null; etiqueta: string; ancho?: string }) {
  return (
    <div className={`flex ${ancho} shrink-0 items-center justify-center gap-1 self-stretch`} title={etiqueta} aria-label={valor ? `${valor} ${etiqueta.toLowerCase()}` : undefined}>
      {valor === null ? null : (
        <>
          <span className="text-black">{icono}</span>
          <span className={`${CLASE_TEXTO} whitespace-nowrap`}>{valor}</span>
        </>
      )}
    </div>
  );
}

/* Cada inmueble va en su caja de 100px: imagen, título, specs, precio y flecha.
 * Toda la caja abre el detalle (clic o Enter/Espacio). En móvil (<md) solo
 * imagen (más pequeña) y título: specs, precio y flecha se ocultan. */
export function CajaInmueble({ inmueble: i, alElegir }: { inmueble: InmueblePublico; alElegir: (i: InmueblePublico) => void }) {
  /* [249A-1] La tabla pide el thumb de 320 px; el modal (máxima
   * resolución) lo pide `ModalDetallePublico` al abrirse. */
  const portada = miniaturaDe(portadaDe(i));
  return (
    <article
      onClick={() => alElegir(i)}
      onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && alElegir(i)}
      tabIndex={0}
      className={`flex ${ALTO_CAJA} cursor-pointer items-center rounded-none ${CLASE_HOVER_CAJA}`}
    >
      {portada ? (
        <img src={portada} alt={i.titulo} loading="lazy" className={`${MARGEN_IMAGEN_CAJA} ${TAMANO_IMAGEN_CAJA} shrink-0 object-cover`} />
      ) : (
        <div className={`${MARGEN_IMAGEN_CAJA} ${TAMANO_IMAGEN_CAJA} shrink-0 ${CLASE_RELLENO_SUAVE}`} aria-hidden />
      )}
      <h2 className={`m-0 flex min-w-0 flex-1 items-center self-stretch px-4 text-left ${CLASE_TEXTO}`}>{i.titulo}</h2>
      {/* Zona central: ocupa el mismo espacio flexible que el título, así el
       * grupo de specs queda centrado entre título y precio en todas las filas
       * (equilibrio automático) y las columnas fijas conservan la alineación. */}
      <div className={`min-w-0 flex-1 items-stretch justify-center self-stretch ${SOLO_ESCRITORIO_FLEX}`}>
        <CeldaSpec icono={<BedDouble className="h-4 w-4" />} valor={i.habitaciones > 0 ? `${i.habitaciones}` : null} etiqueta="Habitaciones" />
        <CeldaSpec icono={<Bath className="h-4 w-4" />} valor={i.banos > 0 ? `${i.banos}` : null} etiqueta="Baños" />
        <CeldaSpec icono={<Ruler className="h-4 w-4" />} valor={i.metros > 0 ? `${i.metros} m²` : null} etiqueta="Metros" ancho={ANCHO_SPEC_LARGO} />
        <CeldaSpec icono={<CarFront className="h-4 w-4" />} valor={i.puestos > 0 ? `${i.puestos}` : null} etiqueta="Puestos" />
      </div>
      <p className={`m-0 ${ANCHO_PRECIO} shrink-0 px-4 text-right ${CLASE_TEXTO} ${SOLO_ESCRITORIO_BLOQUE}`}>{formatearPrecio(i.precio)}</p>
      {/* Botón de detalle: la caja entera ya abre el detalle; la flecha
       * es el affordance visual (no duplica la acción). */}
      <span
        aria-hidden
        className={`mr-4 h-10 w-10 shrink-0 items-center justify-center rounded-none border-0 bg-transparent p-0 ${SOLO_ESCRITORIO_FLEX}`}
      >
        <ArrowRight className={`h-6 w-6 ${CLASE_ACENTO}`} />
      </span>
    </article>
  );
}
