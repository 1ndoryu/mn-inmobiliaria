import { useEffect, useMemo, useRef, useState } from 'react';
import { useInmueblePublico } from '../../../hooks/publica/use-inmueble-publico';
import {
  imagenDeOperacion,
  PRESETS_EXPORTACION,
  recetaVigenteDe,
  resolverReceta,
  type ComposicionPublicidad,
  type FormatoPublicidad,
} from '../../../domain/plantilla-publicidad';
import type { Inmueble } from '../../../domain/inmueble';
import { exportarPublicidad, renderizarPublicidad } from '../../../platform/canvas-publicidad';
import { BOTON_PUBLICIDAD, BOTON_PUBLICIDAD_ACTIVO } from './estilos-publicidad';

const ANCHO_VISTA = 360;

/* Dibuja la imagen en un canvas; `revision` fuerza el redibujado al cambiar el teléfono. */
function Lienzo({ inmueble, comp, revision }: { inmueble: Inmueble; comp: ComposicionPublicidad; revision: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    // `vivo` descarta el dibujo si, mientras carga, ya cambió el formato.
    let vivo = true;
    setError(false);
    renderizarPublicidad(ref.current!, inmueble, comp, ANCHO_VISTA, () => vivo).catch(() => {
      if (vivo) setError(true);
    });
    return () => {
      vivo = false;
    };
  }, [inmueble, comp, revision]);

  if (error) return <p>No se pudo dibujar la imagen.</p>;
  return <canvas ref={ref} className="h-auto w-full max-w-[360px]" />;
}

/* Imagen de publicidad del inmueble: vista previa y descarga en cada formato.
 * El detalle público trae la receta, así que se dibuja igual que en el panel. */
export function ImagenPublicidad({ slug, revision }: { slug: string; revision: number }) {
  const { inmueble, cargando, error } = useInmueblePublico(slug);
  const [formato, setFormato] = useState<FormatoPublicidad | null>(null);
  const [operacion, setOperacion] = useState<'venta' | 'alquiler'>('venta');
  const [bajando, setBajando] = useState(false);
  const [fallo, setFallo] = useState(false);

  const imagen = useMemo(() => (inmueble ? imagenDeOperacion(inmueble, operacion) : null), [inmueble, operacion]);
  const comp = useMemo<ComposicionPublicidad | null>(() => {
    const receta = imagen ? recetaVigenteDe(imagen, imagen.receta) : null;
    if (!imagen || !receta) return null;
    const base = resolverReceta(imagen, receta);
    return formato ? { ...base, formato } : base;
  }, [imagen, formato]);

  if (cargando) return <p>Preparando imagen…</p>;
  if (error || !inmueble || !imagen || !comp) return <p>Este inmueble no tiene imagen de publicidad.</p>;

  const descargar = async (tipo: 'png' | 'jpeg') => {
    setBajando(true);
    setFallo(false);
    try {
      await exportarPublicidad(imagen, comp, tipo);
    } catch {
      setFallo(true);
    } finally {
      setBajando(false);
    }
  };

  return (
    <div className="space-y-2">
      <Lienzo inmueble={imagen} comp={comp} revision={revision} />
      <div className="flex flex-wrap gap-2" role="group" aria-label="Formato de la imagen">
        {PRESETS_EXPORTACION.map((p) => (
          <button
            key={p.formato}
            type="button"
            aria-pressed={comp.formato === p.formato}
            className={comp.formato === p.formato ? BOTON_PUBLICIDAD_ACTIVO : BOTON_PUBLICIDAD}
            onClick={() => setFormato(p.formato)}
          >
            {p.etiqueta}
          </button>
        ))}
      </div>
      {inmueble.operacion === 'ambas' && (
        <div className="flex flex-wrap gap-2" role="group" aria-label="Operación de la imagen">
          {(['venta', 'alquiler'] as const).map((op) => (
            <button
              key={op}
              type="button"
              aria-pressed={operacion === op}
              className={operacion === op ? BOTON_PUBLICIDAD_ACTIVO : BOTON_PUBLICIDAD}
              onClick={() => setOperacion(op)}
            >
              {op === 'venta' ? 'Venta' : 'Alquiler'}
            </button>
          ))}
        </div>
      )}
      <div className="flex gap-2">
        <button type="button" className={BOTON_PUBLICIDAD} disabled={bajando} onClick={() => void descargar('jpeg')}>
          Descargar JPG
        </button>
        <button type="button" className={BOTON_PUBLICIDAD} disabled={bajando} onClick={() => void descargar('png')}>
          Descargar PNG
        </button>
      </div>
      {fallo && <p>Falló la descarga.</p>}
    </div>
  );
}
