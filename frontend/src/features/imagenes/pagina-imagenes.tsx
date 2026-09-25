import { useMemo, useState } from 'react';
import { ChevronLeft, ChevronRight, ImageIcon } from 'lucide-react';
import type { Inmueble } from '@/domain/inmueble';
import type { FotoMejora } from '@/domain/foto-mejora';
import type { InfoReintento } from '@/hooks/mejora/use-cola-mejora';
import { Button } from '@/components/ui/button';
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
  /** ID de fila del original en la API (null en borradores sin subir). */
  idOriginal: string | null;
  /** ID de fila de la mejorada en la API (null si aún no hay). */
  idMejorada: string | null;
}

/* Grupos de inmuebles por página: 4 tarjetas de grupo caben sin scroll
 * eterno con 11 inmuebles (3 páginas); los grupos conservan su orden. */
const GRUPOS_POR_PAGINA = 4;

function Paginador({ pagina, total, alCambiar }: { pagina: number; total: number; alCambiar: (p: number) => void }) {
  if (total <= 1) return null;
  return (
    <div className="flex items-center justify-center gap-2">
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-xs"
        disabled={pagina <= 1}
        onClick={() => alCambiar(pagina - 1)}
      >
        <ChevronLeft className="h-3.5 w-3.5" /> Anterior
      </Button>
      <span className="text-xs text-muted-foreground">
        Página {pagina} de {total}
      </span>
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-xs"
        disabled={pagina >= total}
        onClick={() => alCambiar(pagina + 1)}
      >
        Siguiente <ChevronRight className="h-3.5 w-3.5" />
      </Button>
    </div>
  );
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
  alRestaurar: (inmuebleId: string, orden: number) => Promise<void>;
  ocupado: boolean;
  reintentos?: Record<string, InfoReintento>;
}) {
  const { inmuebles, fotos, alReintentar, alCancelar, alRestaurar, ocupado, reintentos } = props;
  const [pagina, setPagina] = useState(1);

  const grupos = useMemo(() => {
    const grupos: { inmueble: Inmueble; filas: FilaFoto[] }[] = [];
    for (const inmueble of inmuebles) {
      if (inmueble.fotos.length === 0) continue;
      const entradas = fotos.filter((f) => f.inmuebleId === inmueble.id);
      const filas: FilaFoto[] = inmueble.fotos.map((original, orden) => {
        const mejorada = inmueble.mejoradasServidor.find((m) => m.orden === orden) ?? null;
        const mejoradaServidor = mejorada?.url ?? null;
        const idOriginal = inmueble.idsFotos[orden] ?? null;
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
            idOriginal,
            idMejorada: mejorada?.id ?? null,
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
          idOriginal,
          idMejorada: mejorada?.id ?? null,
        };
      });
      grupos.push({ inmueble, filas });
    }
    return grupos.sort((a, b) => b.inmueble.createdAt.localeCompare(a.inmueble.createdAt));
  }, [inmuebles, fotos]);

  const totalPaginas = Math.max(1, Math.ceil(grupos.length / GRUPOS_POR_PAGINA));
  /* Página vigente derivada durante el render (sin efecto): si la lista
   * encoge y la página queda fuera de rango se muestra la última válida. */
  const paginaVigente = Math.min(pagina, totalPaginas);
  const visibles = grupos.slice((paginaVigente - 1) * GRUPOS_POR_PAGINA, paginaVigente * GRUPOS_POR_PAGINA);

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
      <Paginador pagina={paginaVigente} total={totalPaginas} alCambiar={setPagina} />
      {visibles.map(({ inmueble, filas }) => (
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
                idOriginal={fila.idOriginal}
                idServidor={fila.idMejorada}
                alReintentar={alReintentar}
                alCancelar={alCancelar}
                alRestaurar={fila.idMejorada ? () => alRestaurar(inmueble.id, fila.orden) : null}
                ocupado={ocupado || fila.sinEntrada}
                reintento={fila.sinEntrada ? null : (reintentos?.[fila.foto.id] ?? null)}
              />
            ))}
          </div>
        </section>
      ))}
      <Paginador pagina={paginaVigente} total={totalPaginas} alCambiar={setPagina} />
    </div>
  );
}
