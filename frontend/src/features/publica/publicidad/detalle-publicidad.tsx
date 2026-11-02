import { useMemo, useState, type ReactNode } from 'react';
import { ChevronDown, Download, Pencil } from 'lucide-react';
import type { PublicidadPublicaItem } from '../../../domain/redes/publicacion-redes';
import { fotosVisiblesDe, miniaturaDe, type InmueblePublico } from '../../../domain/inmueble';
import { recetaVigenteDe, type RecetaPublicidad } from '../../../domain/plantilla-publicidad';
import { copiarTexto } from '../../../platform/documento';
import { descargarFotoConLogo, descargarFotosZip } from '../../../platform/logo-descarga';
import { guardarRecetaLocal, recetaLocalDe } from '../../../platform/receta-publicidad-local';
import { useInmueblePublico } from '../../../hooks/publica/use-inmueble-publico';
import { ModalEditorPublicidad } from '../../publicidad/modal-editor-publicidad';
import { CLASE_BORDE, CLASE_DIVISOR, CLASE_HOVER_CAJA, CLASE_RELLENO_SUAVE, CLASE_TEXTO, MARGEN_IMAGEN_CAJA, TAMANO_IMAGEN_CAJA } from '../disenno';
import { BOTON_PUBLICIDAD } from './estilos-publicidad';
import { BotonIcono, ImagenPublicidad } from './imagen-publicidad';
import { FotoMarcada } from '../foto-marcada';

const etiquetaHash = (t: string): string => `#${t.replace(/^#/, '')}`;

/* Miniatura de cada fila: mismo tamaño y margen que la de los inmuebles. */
const CLASE_MINIATURA = `${MARGEN_IMAGEN_CAJA} ${TAMANO_IMAGEN_CAJA} shrink-0 overflow-hidden object-cover`;

/* Nombre de archivo a partir del título: minúsculas y guiones. */
const nombreBase = (titulo: string): string => titulo.trim().toLowerCase().replace(/\s+/g, '-');

/* Botón que copia un texto y lo confirma un momento. */
function BotonCopiar({ texto, etiqueta }: { texto: string; etiqueta: string }) {
  const [estado, setEstado] = useState<'reposo' | 'copiado' | 'fallo'>('reposo');
  const copiar = async () => {
    setEstado((await copiarTexto(texto)) ? 'copiado' : 'fallo');
  };
  return (
    <button type="button" className={BOTON_PUBLICIDAD} onClick={() => void copiar()}>
      {estado === 'copiado' ? 'Copiado' : estado === 'fallo' ? 'No se pudo copiar' : etiqueta}
    </button>
  );
}

/* Lista desplegable como la de inmuebles: cada publicación es una fila con su miniatura
 * y al abrirla aparece su imagen (con descarga) y su texto. Varias pueden quedar abiertas. */
function ListaPublicaciones({ encabezado, items }: { encabezado: string; items: { id: string; titulo: string; miniatura?: ReactNode; cuerpo: ReactNode }[] }) {
  const [abiertas, setAbiertas] = useState<ReadonlySet<string>>(() => new Set());
  const alternar = (id: string) =>
    setAbiertas((previo) => {
      const siguiente = new Set(previo);
      if (siguiente.has(id)) siguiente.delete(id);
      else siguiente.add(id);
      return siguiente;
    });

  return (
    <section className="space-y-3">
      <h3 className="text-sm font-semibold">{encabezado}</h3>
      <div className={`w-full divide-y ${CLASE_DIVISOR} border ${CLASE_BORDE}`}>
        {items.map((it) => {
          const abierta = abiertas.has(it.id);
          return (
            <div key={it.id}>
              <button
                type="button"
                aria-expanded={abierta}
                onClick={() => alternar(it.id)}
                className={`flex w-full items-center gap-3 pr-4 text-left ${CLASE_TEXTO} ${CLASE_HOVER_CAJA}`}
              >
                {it.miniatura ?? <div className={`${CLASE_MINIATURA} ${CLASE_RELLENO_SUAVE}`} aria-hidden />}
                <span className="min-w-0 flex-1">{it.titulo}</span>
                <ChevronDown className={`h-5 w-5 shrink-0 transition-transform ${abierta ? 'rotate-180' : ''}`} aria-hidden />
              </button>
              {abierta && <div className="px-4 pb-4">{it.cuerpo}</div>}
            </div>
          );
        })}
      </div>
    </section>
  );
}

/* Publicación de redes del inmueble: las piezas con su descarga y el texto. */
function PublicacionRedes({ material }: { material: PublicidadPublicaItem }) {
  const etiquetas = material.tags.map(etiquetaHash).join(' ');
  const textoCompleto = [material.descripcion, etiquetas].filter(Boolean).join('\n\n');
  return (
    <div className="space-y-3">
      {material.piezas.length > 0 && (
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          {material.piezas.map((p) => (
            <div key={p.id} className="space-y-1">
              <div className="group relative">
                <img src={p.url} alt={`Pieza ${p.formato}`} loading="lazy" className="block w-full" />
                <div className="absolute inset-0 flex items-center justify-center opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
                  <a
                    href={p.url}
                    download
                    target="_blank"
                    rel="noreferrer"
                    aria-label={`Descargar pieza ${p.formato}`}
                    title={`Descargar pieza ${p.formato}`}
                    className={`${BOTON_PUBLICIDAD} flex h-10 w-10 items-center justify-center bg-white p-0`}
                  >
                    <Download className="h-5 w-5" aria-hidden />
                  </a>
                </div>
              </div>
              <p className="text-xs">{p.formato}</p>
            </div>
          ))}
        </div>
      )}
      {textoCompleto && (
        <div className="space-y-2">
          {material.descripcion && <p className="whitespace-pre-line">{material.descripcion}</p>}
          {etiquetas && <p>{etiquetas}</p>}
          <BotonCopiar texto={textoCompleto} etiqueta="Copiar texto" />
        </div>
      )}
    </div>
  );
}

/* Lo que se despliega al abrir un inmueble: fotos (con descarga por foto y «Descargar
 * todas» en ZIP), su imagen de publicidad (con botón de edición) y, si las hay, las
 * publicaciones de redes. */
export function DetalleFotos({ inmueble, material, revision }: { inmueble: InmueblePublico; material?: PublicidadPublicaItem; revision: number }) {
  const { inmueble: detalle, cargando, error } = useInmueblePublico(inmueble.slug);
  /* Receta editada en este navegador: manda sobre la del servidor (que es la del admin). */
  const [recetaNueva, setRecetaNueva] = useState<RecetaPublicidad | null>(() => recetaLocalDe(inmueble.slug));
  const [editando, setEditando] = useState(false);
  const [guardados, setGuardados] = useState(0);
  const [zip, setZip] = useState<'reposo' | 'trabajando' | 'fallo'>('reposo');

  const urls = fotosVisiblesDe(inmueble);
  const base = nombreBase(inmueble.titulo);
  const conReceta = useMemo<InmueblePublico | null>(
    () => (detalle && recetaNueva ? { ...detalle, receta: recetaNueva } : detalle),
    [detalle, recetaNueva],
  );
  const recetaActual = useMemo(() => (conReceta ? recetaVigenteDe(conReceta, conReceta.receta) : null), [conReceta]);

  const descargarZip = async () => {
    setZip('trabajando');
    try {
      await descargarFotosZip(urls, `${base}.zip`);
      setZip('reposo');
    } catch {
      setZip('fallo');
    }
  };

  /* Solo en este navegador: la receta de `/publicidad` no sale al servidor. */
  const guardar = (receta: RecetaPublicidad) => {
    guardarRecetaLocal(inmueble.slug, receta);
    setRecetaNueva(receta);
    setGuardados((n) => n + 1);
    setEditando(false);
  };

  if (cargando) return <p className="p-3 text-sm">Preparando imagen…</p>;
  if (error || !conReceta) return <p className="p-3 text-sm">{error ?? 'No se pudo cargar el inmueble.'}</p>;

  return (
    <div className="space-y-5 p-3">
      <section className="space-y-3">
        <div className="flex flex-wrap items-center gap-3">
          <h3 className="text-sm font-semibold">Fotos</h3>
          <button type="button" className={BOTON_PUBLICIDAD} disabled={zip === 'trabajando' || urls.length === 0} onClick={() => void descargarZip()}>
            {zip === 'trabajando' ? 'Preparando…' : `Descargar todas (${urls.length})`}
          </button>
          {zip === 'fallo' && <span className="text-sm">No se pudo descargar. Inténtalo de nuevo.</span>}
        </div>
        {urls.length === 0 ? (
          <p className="text-sm">Este inmueble no tiene fotos.</p>
        ) : (
          <div className="grid grid-cols-3 gap-1 sm:grid-cols-4 md:grid-cols-6">
            {urls.map((url, i) => (
              <div key={url} className="group relative">
                <FotoMarcada src={miniaturaDe(url)} alt={`${inmueble.titulo}, foto ${i + 1}`} loading="lazy" className="aspect-square w-full" />
                <div className="absolute inset-0 flex items-center justify-center opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
                  <BotonIcono
                    icono={Download}
                    etiqueta={`Descargar foto ${i + 1} con logo`}
                    disabled={zip === 'trabajando'}
                    onClick={() => void descargarFotoConLogo(url, `${base}-${i + 1}.jpg`).catch(() => setZip('fallo'))}
                  />
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="space-y-3">
        <div className="flex flex-wrap items-center gap-3">
          <h3 className="text-sm font-semibold">Imagen de publicidad</h3>
          {recetaActual && (
            <button type="button" className={BOTON_PUBLICIDAD} onClick={() => setEditando(true)}>
              <Pencil className="mr-1 inline h-4 w-4" aria-hidden /> Editar
            </button>
          )}
        </div>
        <ImagenPublicidad inmueble={conReceta} revision={revision + guardados} />
      </section>

      {material && (
        <ListaPublicaciones
          encabezado="Publicaciones"
          items={[
            {
              id: 'redes',
              titulo: 'Publicación de redes',
              miniatura: material.piezas[0] && <img src={material.piezas[0].url} alt="" loading="lazy" className={CLASE_MINIATURA} />,
              cuerpo: <PublicacionRedes material={material} />,
            },
          ]}
        />
      )}

      {editando && recetaActual && (
        <ModalEditorPublicidad
          inmueble={conReceta}
          receta={recetaActual}
          alGuardar={guardar}
          alCerrar={() => setEditando(false)}
        />
      )}
    </div>
  );
}
