import { useEffect, useState } from 'react';
import { usePublicidadPublica } from '../../../hooks/publica/use-publicidad-publica';
import type { PublicidadPublicaItem } from '../../../domain/redes/publicacion-redes';
import { descargarUrl } from '../../../platform/descarga';
import { copiarTexto, fijarEtiquetasPagina, fijarNoIndexar } from '../../../platform/documento';
import { descargarFotoConLogo } from '../../../platform/logo-descarga';
import { origenActual } from '../../../platform/ventana';
import {
  ANCHO_PAGINA,
  BOTON_CONTORNO,
  CLASE_FONDO,
  CLASE_TEXTO,
  CLASE_TINTA,
  CUADRICULA_FOTOS,
  ETIQUETA_TAG,
  IMAGEN_FOTO,
  IMAGEN_PIEZA,
  RELLENO_LATERAL_SITIO,
  RELLENO_VACIO,
} from '../disenno';
import { PiePublico } from '../pie-publico';

/* [28AA-8] Página pública /publicidad: piezas ya publicadas, con descripción y
 * etiquetas para copiar, y fotos y piezas para descargar. No tiene enlace en el
 * menú y pide a los buscadores que no la indexen. */
export function PaginaPublicidad() {
  const { items, cargando, error } = usePublicidadPublica();

  useEffect(() => {
    const restaurarEtiquetas = fijarEtiquetasPagina({
      titulo: 'Publicidad · MN Inmobiliaria',
      descripcion: 'Piezas de publicidad de los inmuebles de MN Inmobiliaria.',
      url: `${origenActual()}/publicidad`,
      imagen: `${origenActual()}/img/logo-mn.svg`,
      datosEstructurados: {
        '@context': 'https://schema.org',
        '@type': 'CollectionPage',
        name: 'Publicidad · MN Inmobiliaria',
      },
    });
    const restaurarRobots = fijarNoIndexar();
    return () => {
      restaurarEtiquetas();
      restaurarRobots();
    };
  }, []);

  return (
    <div className={`flex min-h-screen w-full flex-col ${CLASE_FONDO} ${CLASE_TINTA} ${CLASE_TEXTO}`}>
      <CabeceraSencilla />
      <main className={`mx-auto w-full flex-1 ${ANCHO_PAGINA} ${RELLENO_LATERAL_SITIO} py-6`}>
        {cargando && <p className={RELLENO_VACIO}>Cargando publicidad…</p>}
        {error && (
          <p role="alert" className={RELLENO_VACIO}>
            No se pudo cargar la publicidad: {error}
          </p>
        )}
        {!cargando && !error && items.length === 0 && (
          <p className={RELLENO_VACIO}>Todavía no hay publicidad publicada.</p>
        )}
        <ul className="flex flex-col gap-12">
          {items.map((item) => (
            <li key={item.inmuebleId}>
              <TarjetaPublicidad item={item} />
            </li>
          ))}
        </ul>
      </main>
      <PiePublico />
    </div>
  );
}

function CabeceraSencilla() {
  return (
    <header className={`mx-auto flex w-full ${ANCHO_PAGINA} ${RELLENO_LATERAL_SITIO} py-4`}>
      <a href="/" className="flex items-center gap-3">
        <img src="/img/logo-mn.svg" alt="" width={36} height={36} />
        <span className="font-bold">MN Inmobiliaria</span>
      </a>
    </header>
  );
}

function TarjetaPublicidad({ item }: { item: PublicidadPublicaItem }) {
  const [aviso, setAviso] = useState<string | null>(null);

  /* Copia y descarga comparten el aviso: se ve lo último que pasó. */
  async function copiar(texto: string) {
    const ok = await copiarTexto(texto);
    setAviso(ok ? 'Copiado al portapapeles.' : 'No se pudo copiar. Selecciona el texto a mano.');
  }

  async function bajar(accion: () => Promise<void>) {
    setAviso(null);
    try {
      await accion();
    } catch (e) {
      setAviso(e instanceof Error ? e.message : 'No se pudo descargar.');
    }
  }

  return (
    <article className="flex flex-col gap-6">
      <header className="flex flex-wrap items-baseline justify-between gap-2">
        <h2 className="text-2xl font-bold">{item.titulo}</h2>
        <a href={`/inmueble/${item.slug}`} className="underline">
          Ver inmueble
        </a>
      </header>

      <section className="flex flex-col gap-3">
        <p className="whitespace-pre-line">{item.descripcion}</p>
        <div>
          <button type="button" className={BOTON_CONTORNO} onClick={() => copiar(item.descripcion)}>
            Copiar descripción
          </button>
        </div>
      </section>

      {item.tags.length > 0 && (
        <section className="flex flex-col gap-3">
          <ul className="flex flex-wrap gap-2">
            {item.tags.map((tag) => (
              <li key={tag} className={ETIQUETA_TAG}>
                {tag}
              </li>
            ))}
          </ul>
          <div>
            <button type="button" className={BOTON_CONTORNO} onClick={() => copiar(item.tags.join(', '))}>
              Copiar etiquetas
            </button>
          </div>
        </section>
      )}

      {item.piezas.length > 0 && (
        <section className="flex flex-col gap-3">
          <h3 className="font-bold">Piezas</h3>
          <ul className={CUADRICULA_FOTOS}>
            {item.piezas.map((pieza) => (
              <li key={pieza.id} className="flex flex-col gap-2">
                <img src={pieza.url} alt={`Pieza ${pieza.formato} de ${item.titulo}`} loading="lazy" className={IMAGEN_PIEZA} />
                <button
                  type="button"
                  className={BOTON_CONTORNO}
                  onClick={() =>
                    bajar(() =>
                      descargarUrl(pieza.url, `${item.slug}-${pieza.formato.replace(':', 'x')}.${extension(pieza.url)}`),
                    )
                  }
                >
                  Descargar {pieza.formato}
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}

      {item.fotos.length > 0 && (
        <section className="flex flex-col gap-3">
          <h3 className="font-bold">Fotos</h3>
          <ul className={CUADRICULA_FOTOS}>
            {item.fotos.map((foto, n) => (
              <li key={foto} className="flex flex-col gap-2">
                <img src={foto} alt={`Foto ${n + 1} de ${item.titulo}`} loading="lazy" className={IMAGEN_FOTO} />
                <button
                  type="button"
                  className={BOTON_CONTORNO}
                  onClick={() => bajar(() => descargarFotoConLogo(foto, `${item.slug}-foto-${n + 1}.jpg`))}
                >
                  Descargar con logo
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}

      {aviso && (
        <p role="status" className="text-sm">
          {aviso}
        </p>
      )}
    </article>
  );
}

/* Extensión del archivo según la URL de la pieza; JPEG si no se reconoce. */
function extension(url: string): string {
  return url.split('?')[0].match(/\.(\w+)$/)?.[1] ?? 'jpg';
}
