import { useEffect, useState } from 'react';
import { usePublicidadPublica } from '../../../hooks/publica/use-publicidad-publica';
import { fijarNoIndexar } from '../../../platform/documento';
import { CabeceraAcciones } from '../inmueble/cabecera-acciones';
import { ANCHO_PAGINA, CLASE_BORDE, CLASE_FONDO, ESPACIO_CABECERA_FIJA, RELLENO_LATERAL_SITIO, RELLENO_VACIO } from '../disenno';
import { PiePublico } from '../pie-publico';
import { TarjetaPublicidad } from './tarjeta-publicidad';
import { TelefonoPublicidad } from './telefono-publicidad';

/* Material de difusión para asesores (`/publicidad`): sin enlace en el menú y
 * con `noindex`. Solo muestra lo que el servidor ya considera público. */
export function PaginaPublicidad() {
  const { items, cargando, error } = usePublicidadPublica();
  /* Al guardar el teléfono sube la revisión y las imágenes se redibujan. */
  const [revision, setRevision] = useState(0);

  useEffect(() => fijarNoIndexar(), []);

  return (
    <main className={`flex min-h-dvh flex-col items-center justify-start ${CLASE_FONDO} font-soehne text-black pt-4 pb-16`}>
      <div className={`mx-auto w-full ${ANCHO_PAGINA} ${RELLENO_LATERAL_SITIO}`}>
        <CabeceraAcciones />
        <div aria-hidden className={ESPACIO_CABECERA_FIJA} />
        <h1 className="mb-4 text-3xl font-bold tracking-tight">Publicidad</h1>
        <div className="mb-6">
          <TelefonoPublicidad alGuardar={() => setRevision((r) => r + 1)} />
        </div>
        {cargando ? (
          <p className={`text-center ${RELLENO_VACIO}`}>Cargando material…</p>
        ) : error ? (
          <p className={`border ${CLASE_BORDE} text-center ${RELLENO_VACIO}`}>{error}</p>
        ) : items.length === 0 ? (
          <p className={`border ${CLASE_BORDE} text-center ${RELLENO_VACIO}`}>Todavía no hay material publicado.</p>
        ) : (
          <div className="space-y-6">
            {items.map((item) => (
              <TarjetaPublicidad key={item.inmuebleId} item={item} revision={revision} />
            ))}
          </div>
        )}
      </div>
      <PiePublico />
    </main>
  );
}
