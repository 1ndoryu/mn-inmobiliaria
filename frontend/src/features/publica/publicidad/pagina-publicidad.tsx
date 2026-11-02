import { useEffect, useMemo, useState } from 'react';
import type { InmueblePublico } from '../../../domain/inmueble';
import { usePublica } from '../../../hooks/publica/use-publica';
import { usePublicidadPublica } from '../../../hooks/publica/use-publicidad-publica';
import { fijarNoIndexar } from '../../../platform/documento';
import { CabeceraAcciones } from '../inmueble/cabecera-acciones';
import { CajaInmueble } from '../lista/caja-inmueble';
import { ANCHO_PAGINA, CLASE_BORDE, CLASE_DIVISOR, CLASE_FONDO, ESPACIO_CABECERA_FIJA, RELLENO_LATERAL_SITIO, RELLENO_VACIO } from '../disenno';
import { PiePublico } from '../pie-publico';
import { DetalleFotos } from './detalle-publicidad';
import { TelefonoPublicidad } from './telefono-publicidad';

/* Material de difusión para asesores (`/publicidad`): sin enlace en el menú y
 * con `noindex`. Lista todos los publicados con la caja de la página de inicio;
 * al abrir una se despliegan sus fotos y, si tiene publicación de redes, su material. */
export function PaginaPublicidad() {
  const { inmuebles, cargando, error } = usePublica();
  const { items } = usePublicidadPublica();
  /* Al guardar el teléfono sube la revisión y las imágenes se redibujan. */
  const [revision, setRevision] = useState(0);
  const [abiertos, setAbiertos] = useState<ReadonlySet<string>>(() => new Set());
  const materialPorInmueble = useMemo(() => new Map(items.map((m) => [m.inmuebleId, m])), [items]);

  useEffect(() => fijarNoIndexar(), []);

  /* Abre o cierra el despliegue de un inmueble; varios pueden quedar abiertos. */
  const alternar = (inmueble: InmueblePublico) =>
    setAbiertos((previo) => {
      const siguiente = new Set(previo);
      if (siguiente.has(inmueble.id)) siguiente.delete(inmueble.id);
      else siguiente.add(inmueble.id);
      return siguiente;
    });

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
          <p className={`text-center ${RELLENO_VACIO}`}>Cargando inmuebles…</p>
        ) : error ? (
          <p className={`border ${CLASE_BORDE} text-center ${RELLENO_VACIO}`}>{error}</p>
        ) : inmuebles.length === 0 ? (
          <p className={`border ${CLASE_BORDE} text-center ${RELLENO_VACIO}`}>Sin propiedades publicadas.</p>
        ) : (
          <div className={`w-full divide-y ${CLASE_DIVISOR} border ${CLASE_BORDE}`}>
            {inmuebles.map((i) => (
              <div key={i.id}>
                <CajaInmueble inmueble={i} alElegir={alternar} />
                {abiertos.has(i.id) && <DetalleFotos inmueble={i} material={materialPorInmueble.get(i.id)} revision={revision} />}
              </div>
            ))}
          </div>
        )}
      </div>
      <PiePublico />
    </main>
  );
}
