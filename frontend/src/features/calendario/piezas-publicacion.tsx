/* [28AA-6] Piezas por formato (1:1, 3:4, 4:5, 9:16) de una publicación. La foto se
 * recorta en el navegador; el servidor recibe ya el JPEG en proporción. */
import { Trash2 } from 'lucide-react';
import { useState } from 'react';
import { Button, buttonVariants } from '@/components/ui/button';
import { urlFotoSolicitud } from '@/data/publicar/api';
import { borrarPieza, subirPieza } from '@/data/publicaciones/api';
import { FORMATOS_PIEZA, type FormatoPieza, type Pieza, type Publicacion } from '@/domain/redes/publicacion-redes';
import { mensajeError } from '@/hooks/publicaciones/use-calendario';
import { cn } from '@/lib/utils';
import { cargarImagenArchivo, recortarAFormato } from '@/platform/imagen';

/* Proporción de cada formato en clases (Tailwind no admite clases armadas al vuelo). */
const CLASE_PROPORCION: Record<FormatoPieza, string> = {
  '1:1': 'aspect-square',
  '3:4': 'aspect-[3/4]',
  '4:5': 'aspect-[4/5]',
  '9:16': 'aspect-[9/16]',
};

/* La más reciente gana si una caída dejó dos del mismo formato (el servidor ordena por creación). */
function piezaDe(piezas: Pieza[], formato: FormatoPieza): Pieza | undefined {
  return piezas.filter((p) => p.formato === formato).pop();
}

interface PiezasPublicacionProps {
  publicacion: Publicacion;
  /* Publicada: las piezas ya no se cambian. */
  bloqueado: boolean;
  alActualizar: (publicacion: Publicacion) => void;
}

export function PiezasPublicacion({ publicacion, bloqueado, alActualizar }: PiezasPublicacionProps) {
  const [ocupado, setOcupado] = useState<FormatoPieza | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function subir(formato: FormatoPieza, archivo: File) {
    setOcupado(formato);
    setError(null);
    try {
      const blob = await recortarAFormato(await cargarImagenArchivo(archivo), formato);
      const nueva = await subirPieza(publicacion.id, blob, `pieza-${formato.replace(':', 'x')}.jpg`);
      /* Primero entra la nueva y después salen las antiguas del mismo formato:
       * la BD admite varias por formato, así que nunca queda el hueco sin pieza. */
      const antiguas = publicacion.piezas.filter((p) => p.formato === formato);
      alActualizar({
        ...publicacion,
        piezas: [...publicacion.piezas.filter((p) => p.formato !== formato), nueva],
      });
      for (const vieja of antiguas) await borrarPieza(vieja.id);
    } catch (e) {
      setError(mensajeError(e, 'No se pudo subir la pieza.'));
    } finally {
      setOcupado(null);
    }
  }

  async function quitar(pieza: Pieza) {
    setError(null);
    try {
      await borrarPieza(pieza.id);
      alActualizar({ ...publicacion, piezas: publicacion.piezas.filter((p) => p.id !== pieza.id) });
    } catch (e) {
      setError(mensajeError(e, 'No se pudo quitar la pieza.'));
    }
  }

  const ocupadoTodo = ocupado !== null;
  return (
    <div className="flex flex-col gap-2">
      {error && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        {FORMATOS_PIEZA.map((formato) => {
          const pieza = piezaDe(publicacion.piezas, formato);
          return (
            <div key={formato} className="flex flex-col gap-2 rounded-md border p-2">
              <span className="text-xs font-medium">{formato}</span>
              <div
                className={cn('flex items-center justify-center overflow-hidden rounded bg-muted', CLASE_PROPORCION[formato])}
              >
                {pieza ? (
                  <img src={urlFotoSolicitud(pieza.url)} alt={`Pieza ${formato}`} className="size-full object-cover" />
                ) : (
                  <span className="text-xs text-muted-foreground">Sin pieza</span>
                )}
              </div>
              <div className="flex gap-1">
                <label
                  className={cn(
                    buttonVariants({ variant: 'outline', size: 'sm' }),
                    'flex-1 cursor-pointer',
                    (bloqueado || ocupadoTodo) && 'pointer-events-none opacity-50',
                  )}
                >
                  {ocupado === formato ? 'Subiendo…' : pieza ? 'Cambiar' : 'Subir'}
                  <input
                    type="file"
                    accept="image/*"
                    className="sr-only"
                    disabled={bloqueado || ocupadoTodo}
                    onChange={(e) => {
                      const archivo = e.target.files?.[0];
                      e.target.value = '';
                      if (archivo) void subir(formato, archivo);
                    }}
                  />
                </label>
                {pieza && !bloqueado && (
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    onClick={() => void quitar(pieza)}
                    disabled={ocupadoTodo}
                    aria-label={`Quitar pieza ${formato}`}
                  >
                    <Trash2 />
                  </Button>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
