import { useMemo } from 'react';
import { ImageIcon } from 'lucide-react';
import type { Inmueble } from '@/domain/inmueble';
import type { FotoMejora } from '@/domain/foto-mejora';
import type { InfoReintento } from '@/hooks/mejora/use-cola-mejora';
import { TarjetaFotoMejora } from './tarjeta-foto-mejora';

/* Misma foto con distinta base (absoluta/relativa): se compara por ruta. */
function mismaRuta(a: string, b: string): boolean {
  if (a === b) return true;
  try {
    return new URL(a).pathname === new URL(b).pathname;
  } catch {
    return false;
  }
}

interface FilaFoto {
  key: string;
  orden: number;
  foto: FotoMejora;
  /** Sin entrada local (importación aún en curso): botones deshabilitados. */
  sinEntrada: boolean;
}

/* Página lateral de imágenes: grupos compactos por inmueble, fotos en el
 * orden de subida. Fuente de verdad del render: `inmueble.fotos` (servidor,
 * idéntica en todos los navegadores). La entrada local solo aporta estado
 * de cola (`estado`, `intentos`, `jobId`); una original muerta en IndexedDB
 * (404 tras borrar o re-subir) ya nunca se renderiza: la purga la elimina. */
export function PaginaImagenes(props: {
  inmuebles: Inmueble[];
  fotos: FotoMejora[];
  alReintentar: (foto: FotoMejora) => void;
  alCancelar: (foto: FotoMejora) => void;
  ocupado: boolean;
  reintentos?: Record<string, InfoReintento>;
}) {
  const { inmuebles, fotos, alReintentar, alCancelar, ocupado, reintentos } = props;

  const grupos = useMemo(() => {
    const grupos: { inmueble: Inmueble; filas: FilaFoto[] }[] = [];
    for (const inmueble of inmuebles) {
      if (inmueble.fotos.length === 0) continue;
      const entradas = fotos.filter((f) => f.inmuebleId === inmueble.id);
      const filas: FilaFoto[] = inmueble.fotos.map((original, orden) => {
        const mejoradaServidor = inmueble.mejoradasServidor.find((m) => m.orden === orden)?.url ?? null;
        const entrada =
          entradas.find((e) => e.orden === orden && mismaRuta(e.original, original)) ??
          entradas.find((e) => e.orden === orden) ??
          null;
        if (entrada) {
          return {
            key: entrada.id,
            orden,
            foto: { ...entrada, original, mejorada: mejoradaServidor ?? entrada.mejorada },
            sinEntrada: false,
          };
        }
        return {
          key: `pendiente-${inmueble.id}-${orden}`,
          orden,
          foto: {
            id: `pendiente-${inmueble.id}-${orden}`,
            inmuebleId: inmueble.id,
            orden,
            original,
            mejorada: mejoradaServidor,
            estado: mejoradaServidor ? 'lista' : 'pendiente',
            intentos: 0,
            jobId: null,
            error: null,
            createdAt: inmueble.createdAt,
            updatedAt: inmueble.createdAt,
          },
          sinEntrada: true,
        };
      });
      grupos.push({ inmueble, filas });
    }
    return grupos.sort((a, b) => b.inmueble.createdAt.localeCompare(a.inmueble.createdAt));
  }, [inmuebles, fotos]);

  if (grupos.length === 0) {
    return (
      <div className="flex flex-col items-center gap-2 rounded-lg border border-dashed py-16 text-center">
        <ImageIcon className="h-8 w-8 text-muted-foreground" />
        <p className="font-medium">Aún no hay fotos para mejorar</p>
        <p className="max-w-sm text-sm text-muted-foreground">
          Sube fotos en cualquier inmueble y aquí aparecerán ordenadas con su copia original y su versión mejorada.
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      {grupos.map(({ inmueble, filas }) => (
        <section key={inmueble.id} className="flex flex-col gap-3">
          <div className="flex items-baseline gap-2">
            <h2 className="text-base font-semibold">{inmueble.titulo || 'Sin título'}</h2>
            <span className="text-xs text-muted-foreground">
              {inmueble.ubicacion || '—'} · {filas.length} foto{filas.length === 1 ? '' : 's'}
            </span>
          </div>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {filas.map((fila) => (
              <TarjetaFotoMejora
                key={fila.key}
                foto={fila.foto}
                alReintentar={alReintentar}
                alCancelar={alCancelar}
                ocupado={ocupado || fila.sinEntrada}
                reintento={fila.sinEntrada ? null : (reintentos?.[fila.foto.id] ?? null)}
              />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
