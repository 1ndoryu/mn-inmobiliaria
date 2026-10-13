import { useEffect, useMemo } from 'react';
import { useInmueblePublico } from '../../../hooks/publica/use-inmueble-publico';
import { usePublica } from '../../../hooks/publica/use-publica';
import { fijarEtiquetasPagina } from '../../../platform/documento';
import { irA, origenActual } from '../../../platform/ventana';
import { CabeceraAcciones } from './cabecera-acciones';
import { ContenidoInmueble } from '../detalle/contenido-inmueble';
import { CajaInmueble } from '../lista/caja-inmueble';
import { ANCHO_PAGINA, CLASE_BORDE, CLASE_DIVISOR, CLASE_FONDO, ESPACIO_CABECERA_FIJA, RELLENO_LATERAL_SITIO, RELLENO_VACIO } from '../disenno';
import { PiePublico } from '../pie-publico';
import { rutaInmueble } from './ruta-inmueble';
import { datosSeoInmueble } from './seo-inmueble';
import { inmueblesSimilares } from './similares';

/* Página propia de un inmueble (`/inmueble/<slug>`): mismo diseño que el
 * modal, con cabecera sencilla, inmuebles similares al pie y SEO por página. */
export function PaginaInmueble({ slug }: { slug: string }) {
  const { inmueble, cargando, noEncontrado, error } = useInmueblePublico(slug);
  const { inmuebles } = usePublica();
  const similares = useMemo(() => (inmueble ? inmueblesSimilares(inmuebles, inmueble) : []), [inmuebles, inmueble]);

  useEffect(() => {
    if (!inmueble) return;
    return fijarEtiquetasPagina(datosSeoInmueble(inmueble, origenActual()));
  }, [inmueble]);

  return (
    <main className={`flex min-h-dvh flex-col items-center justify-start ${CLASE_FONDO} font-soehne text-black pt-4 pb-16`}>
      <div className={`mx-auto w-full ${ANCHO_PAGINA} ${RELLENO_LATERAL_SITIO}`}>
        <CabeceraAcciones />
        <div aria-hidden className={ESPACIO_CABECERA_FIJA} />
        <a href="/" className="mb-3 inline-block text-sm underline-offset-4 hover:underline">
          ← Todos los inmuebles
        </a>
        {cargando ? (
          <p className={`text-center ${RELLENO_VACIO}`}>Cargando inmueble…</p>
        ) : noEncontrado ? (
          <p className={`border ${CLASE_BORDE} text-center ${RELLENO_VACIO}`}>Este inmueble ya no está disponible.</p>
        ) : error ? (
          <p className={`border ${CLASE_BORDE} text-center ${RELLENO_VACIO}`}>{error}</p>
        ) : (
          inmueble && (
            <>
              <div className={`border ${CLASE_BORDE}`}>
                <ContenidoInmueble inmueble={inmueble} tituloComo="pagina" />
              </div>
              {similares.length > 0 && (
                <section aria-labelledby="inmuebles-similares" className="mt-10">
                  <h2 id="inmuebles-similares" className="mb-3 text-xl font-normal">
                    Inmuebles similares
                  </h2>
                  <div className={`w-full divide-y ${CLASE_DIVISOR} border ${CLASE_BORDE}`}>
                    {similares.map((s) => (
                      <CajaInmueble key={s.id} inmueble={s} alElegir={(x) => irA(rutaInmueble(x.slug))} />
                    ))}
                  </div>
                </section>
              )}
            </>
          )
        )}
      </div>
      <PiePublico />
    </main>
  );
}
