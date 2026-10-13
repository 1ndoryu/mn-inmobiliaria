import { useEffect, useMemo, useState } from 'react';
import { Bath, BedDouble, CarFront, ChevronLeft, ChevronRight, Link2, MapPin, MessageCircle, Ruler } from 'lucide-react';
import type { InmueblePublico } from '../../../domain/inmueble';
import { fotosVisiblesDe } from '../../../domain/inmueble';
import { recetaVigenteDe, resolverReceta } from '../../../domain/plantilla-publicidad';
import { renderizarPublicidad } from '@/platform/canvas-publicidad';
import { crearLienzo } from '@/platform/imagen';
import { copiarTexto } from '@/platform/documento';
import { temporizar } from '@/platform/ventana';
import { enlaceWhatsApp, mensajePropiedad } from '../../../platform/whatsapp';
import { DialogTitle } from '@/components/ui/dialog';
import { CLASE_ACENTO, CLASE_ACTIVO, CLASE_BORDE, CLASE_MINIATURA_ACTIVA, CLASE_MINIATURA_REPOSO, CLASE_RELLENO_SUAVE } from '../disenno';
import { formatearPrecio } from '../formato';
import { enlaceInmueble } from '../inmueble/ruta-inmueble';
import { ButtonPlano } from '@/components/ui/button';

/* Contenido del detalle de un inmueble: galería, datos, precio, WhatsApp y
 * enlace compartible. Lo usan el modal de la lista (`tituloComo="dialogo"`,
 * dentro de `Dialog`) y la página propia `/inmueble/<slug>` (`"pagina"`). */
export function ContenidoInmueble({ inmueble: i, tituloComo }: { inmueble: InmueblePublico; tituloComo: 'dialogo' | 'pagina' }) {
  const [indice, setIndice] = useState(0);
  /* Galería pública: cada foto en su versión mejorada cuando existe (la
   * primera es la principal). Antes mostraba siempre los originales. */
  const visibles = fotosVisiblesDe(i);
  /* Primera imagen = la publicitaria con la receta elegida en admin
   * (traída del servidor; sin receta = automática: portada de fondo +
   * 2ª/3ª en círculos). Se genera en memoria al abrir; si falla, queda solo fotos. */
  const receta = useMemo(() => recetaVigenteDe(i, i.receta), [i]);
  const comp = useMemo(() => (receta ? resolverReceta(i, receta) : null), [i, receta]);
  /* El marco copia el aspect del formato publicitario SOLO en tablet/PC
   * (sm+); en móvil conserva el alto fijo anterior (h-72). */
  const ASPECTO_SM = { 'post-3-4': 'sm:aspect-[3/4]', 'post-4-5': 'sm:aspect-[4/5]', 'cuadrado-1-1': 'sm:aspect-square' } as const;
  const aspectoSm = comp ? ASPECTO_SM[comp.formato] : 'sm:aspect-[3/4]';
  const [publi, setPubli] = useState<string | null>(null);
  useEffect(() => {
    if (!comp) return;
    let viva = true;
    /* [08AA-23] Lienzo vía plataforma (sin `document` directo aquí). */
    const lienzo = crearLienzo();
    /* HD: a 1080px de ancho (3:4 -> 1080x1440); las fotos ya van a
     * resolución completa del servidor. */
    renderizarPublicidad(lienzo, i, comp, 1080)
      .then(() => {
        if (viva) setPubli(lienzo.toDataURL('image/jpeg', 0.92));
      })
      .catch(() => {});
    return () => {
      viva = false;
    };
  }, [i, comp]);
  const galeria = publi ? [publi, ...visibles] : visibles;
  const total = galeria.length;
  const actual = total === 0 ? null : galeria[((indice % total) + total) % total];
  const irA = (n: number) => setIndice(((n % total) + total) % total);

  /* Enlace compartible: el texto confirma 2 s y vuelve a "Copiar enlace". */
  const [copiado, setCopiado] = useState<boolean | null>(null);
  useEffect(() => {
    if (copiado === null) return;
    return temporizar(2000, () => setCopiado(null));
  }, [copiado]);
  const copiarEnlace = async () => setCopiado(await copiarTexto(enlaceInmueble(i.slug)));

  const claseTitulo = `font-soehne leading-tight font-normal ${tituloComo === 'pagina' ? 'text-2xl md:text-3xl' : 'text-xl'}`;

  return (
    <article className="flex flex-col font-normal sm:flex-row">
      <div
        className={`group relative h-72 w-full shrink-0 border-b sm:h-auto sm:w-[500px] sm:border-r sm:border-b-0 ${aspectoSm} ${CLASE_BORDE} ${CLASE_RELLENO_SUAVE}`}
      >
        {actual ? (
          <img src={actual} alt={i.titulo} className="absolute inset-0 h-full w-full object-contain" />
        ) : (
          <span className="absolute inset-0 flex items-center justify-center text-sm text-black/60">Sin foto</span>
        )}
        {total > 1 && (
          <>
            <ButtonPlano
              type="button"
              aria-label="Foto anterior"
              onClick={() => irA(indice - 1)}
              className="absolute top-1/2 left-2 -translate-y-1/2 rounded-none bg-black/60 p-1 hover:bg-black"
            >
              <ChevronLeft className={`h-5 w-5 ${CLASE_ACENTO}`} />
            </ButtonPlano>
            <ButtonPlano
              type="button"
              aria-label="Foto siguiente"
              onClick={() => irA(indice + 1)}
              className="absolute top-1/2 right-2 -translate-y-1/2 rounded-none bg-black/60 p-1 hover:bg-black"
            >
              <ChevronRight className={`h-5 w-5 ${CLASE_ACENTO}`} />
            </ButtonPlano>
            <div className="absolute inset-x-2 bottom-2 flex justify-center gap-1 opacity-0 transition group-hover:opacity-100">
              {galeria.map((f, idx) => (
                <ButtonPlano
                  key={`${idx}-${f.length}-${f.slice(-16)}`}
                  type="button"
                  aria-label={`Ver foto ${idx + 1}`}
                  onClick={() => irA(idx)}
                  className={`rounded-none p-0 ${idx === ((indice % total) + total) % total ? CLASE_MINIATURA_ACTIVA : CLASE_MINIATURA_REPOSO}`}
                >
                {/* [249A-1] La tira carga miniaturas en diferido: la foto a
                  * máxima resolución solo se pide al verla en grande (la
                  * principal al abrir, las demás al navegar). */}
                  <img src={f} alt="" loading="lazy" decoding="async" className="h-12 w-16 rounded-none object-cover" />
                </ButtonPlano>
              ))}
            </div>
          </>
        )}
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-4 p-6 font-normal">
        {tituloComo === 'pagina' ? (
          <h1 className={claseTitulo}>{i.titulo}</h1>
        ) : (
          <DialogTitle className={claseTitulo}>{i.titulo}</DialogTitle>
        )}
        {i.ubicacion && (
          <p className="flex items-center gap-1 text-sm font-normal text-black/70">
            <MapPin className="h-4 w-4" /> {i.ubicacion}{i.residencia ? ` · ${i.residencia}` : ''}
          </p>
        )}
        <div className="flex items-center gap-4 text-sm font-normal">
          {i.habitaciones > 0 && (
            <span className="flex items-center gap-1">
              <BedDouble className="h-4 w-4" /> {i.habitaciones}
            </span>
          )}
          {i.banos > 0 && (
            <span className="flex items-center gap-1">
              <Bath className="h-4 w-4" /> {i.banos}
            </span>
          )}
          {i.metros > 0 && (
            <span className="flex items-center gap-1">
              <Ruler className="h-4 w-4" /> {i.metros} m²
            </span>
          )}
          {i.puestos > 0 && (
            <span className="flex items-center gap-1">
              <CarFront className="h-4 w-4" /> {i.puestos}
            </span>
          )}
        </div>
        {i.descripcion && <p className="text-sm leading-relaxed font-normal whitespace-pre-line">{i.descripcion}</p>}
        <p className="text-lg font-normal">
          {formatearPrecio(i.precio)} <span className="text-sm text-black/60">{i.operacion}</span>
        </p>
        {/* En local y en producción abre WhatsApp con el interés ya
          * escrito (título, ubicación y precio de esta propiedad). */}
        <a
          href={enlaceWhatsApp(mensajePropiedad(i.titulo, i.ubicacion, formatearPrecio(i.precio), i.operacion))}
          target="_blank"
          rel="noreferrer"
          className={`flex items-center justify-center gap-2 rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 text-sm font-normal`}
        >
          <MessageCircle className="h-4 w-4" />
          Me interesa esta propiedad
        </a>
        <ButtonPlano
          type="button"
          onClick={copiarEnlace}
          className={`flex items-center justify-center gap-2 rounded-none border ${CLASE_BORDE} px-4 py-2 text-sm font-normal`}
        >
          <Link2 className="h-4 w-4" />
          <span aria-live="polite">
            {copiado === null ? 'Copiar enlace' : copiado ? 'Enlace copiado' : 'No se pudo copiar'}
          </span>
        </ButtonPlano>
      </div>
    </article>
  );
}
