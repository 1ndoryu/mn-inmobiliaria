import { useEffect, useMemo, useRef, useState } from 'react';
import { Download } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import type { InmueblePublico, Inmueble } from '../../../domain/inmueble';
import {
  imagenDeOperacion,
  PRESETS_EXPORTACION,
  recetaVigenteDe,
  resolverReceta,
  type ComposicionPublicidad,
} from '../../../domain/plantilla-publicidad';
import { exportarPublicidad, renderizarPublicidad } from '../../../platform/canvas-publicidad';
import { BOTON_PUBLICIDAD, BOTON_PUBLICIDAD_ACTIVO } from './estilos-publicidad';

const ANCHO_VISTA = 360;

/* Red social para la que suele quedar mejor cada formato; va bajo la imagen para elegir cuál bajar. */
const USO_RED: Record<string, string> = {
  'post-3-4': 'Mejor para Instagram',
  'cuadrado-1-1': 'Mejor para Facebook',
  'post-4-5': 'Uso general',
};

/* Botón solo con icono, centrado sobre la imagen. El nombre va en aria-label y title. */
export function BotonIcono({ icono: Icono, etiqueta, onClick, disabled }: {
  icono: LucideIcon;
  etiqueta: string;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={etiqueta}
      title={etiqueta}
      disabled={disabled}
      onClick={onClick}
      className={`${BOTON_PUBLICIDAD} flex h-10 w-10 items-center justify-center bg-white p-0`}
    >
      <Icono className="h-5 w-5" aria-hidden />
    </button>
  );
}

/* Dibuja la imagen en un canvas; `revision` fuerza el redibujado al cambiar el teléfono. */
export function Lienzo({ inmueble, comp, revision }: { inmueble: Inmueble; comp: ComposicionPublicidad; revision: number }) {
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

  if (error) return <p className="text-sm">No se pudo dibujar la imagen.</p>;
  return <canvas ref={ref} className="block h-auto w-full" />;
}

/* Imagen dibujada con su descarga JPG centrada encima. La usan los formatos de la página
 * y los ejemplos de publicación del detalle. */
export function ImagenDescargable({ imagen, comp, revision, etiqueta }: {
  imagen: Inmueble;
  comp: ComposicionPublicidad;
  revision: number;
  etiqueta: string;
}) {
  const [bajando, setBajando] = useState(false);
  const [fallo, setFallo] = useState(false);

  const descargar = async () => {
    setBajando(true);
    setFallo(false);
    try {
      await exportarPublicidad(imagen, comp, 'jpeg');
    } catch {
      setFallo(true);
    } finally {
      setBajando(false);
    }
  };

  return (
    <div className="space-y-1">
      <div className="group relative">
        <Lienzo inmueble={imagen} comp={comp} revision={revision} />
        <div className="absolute inset-0 flex items-center justify-center opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
          <BotonIcono icono={Download} etiqueta={`Descargar ${etiqueta} en JPG`} disabled={bajando} onClick={() => void descargar()} />
        </div>
      </div>
      {fallo && <p className="text-xs">Falló la descarga.</p>}
    </div>
  );
}

/* Un formato: su imagen con el nombre y el uso recomendado debajo. */
function FormatoPublicidad({ imagen, comp, revision, etiqueta, uso }: {
  imagen: Inmueble;
  comp: ComposicionPublicidad;
  revision: number;
  etiqueta: string;
  uso: string;
}) {
  return (
    <div className="space-y-1">
      <ImagenDescargable imagen={imagen} comp={comp} revision={revision} etiqueta={etiqueta} />
      <p className="text-xs font-semibold">{etiqueta}</p>
      <p className="text-xs">{uso}</p>
    </div>
  );
}

/* Imagen de publicidad del inmueble en todos los formatos (4:5, 3:4 y 1:1).
 * Recibe el detalle público con su receta; la vista previa se dibuja igual que en el panel. */
export function ImagenPublicidad({ inmueble, revision }: { inmueble: InmueblePublico; revision: number }) {
  const [operacion, setOperacion] = useState<'venta' | 'alquiler'>('venta');
  const imagen = useMemo(() => imagenDeOperacion(inmueble, operacion), [inmueble, operacion]);
  const receta = useMemo(() => recetaVigenteDe(imagen, imagen.receta), [imagen]);

  if (!receta) return <p className="text-sm">Este inmueble no tiene imagen de publicidad.</p>;
  const base = resolverReceta(imagen, receta);

  return (
    <div className="space-y-2">
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
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        {PRESETS_EXPORTACION.map((p) => (
          <FormatoPublicidad
            key={`${operacion}-${p.formato}`}
            imagen={imagen}
            comp={{ ...base, formato: p.formato }}
            revision={revision}
            etiqueta={p.etiqueta}
            uso={USO_RED[p.formato]}
          />
        ))}
      </div>
    </div>
  );
}
