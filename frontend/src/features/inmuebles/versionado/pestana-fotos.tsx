/* [30AA-1] Fotos variante: la MISMA foto repetida desde otro ángulo, para
 * publicar el inmueble en otra cuenta de Facebook sin que se detecte como
 * duplicado. Se generan con el backend de mejora (Gemini web) que ya usa la
 * app: el prompt de variante vive en la configuración de mejora. */
import { ImagePlus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { FotoVariante } from '@/domain/versiones';

interface Props {
  pares: { orden: number; original: string; variante: FotoVariante | null }[];
  progreso: { total: number; hechas: number; actual: number | null; detalle: string | null } | null;
  ocupado: boolean;
  alGenerar: () => void;
  alBorrar: (orden: number) => void;
}

export function PestanaFotos({ pares, progreso, ocupado, alGenerar, alBorrar }: Props) {
  const hechas = pares.filter((p) => p.variante !== null).length;
  const pendientes = pares.length - hechas;

  return (
    <div className="grid gap-3">
      <p className="rounded-md border bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
        Cada variante se genera a partir de la foto ya mejorada y debe parecer otra toma del mismo sitio: cambia el
        ángulo y la altura, nunca lo que se ve. Va de una en una y respeta las pausas del backend.
      </p>

      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" disabled={ocupado || progreso !== null || pendientes === 0} onClick={alGenerar}>
          <ImagePlus /> {pendientes === 0 ? 'Todas generadas' : `Generar ${pendientes} variante${pendientes === 1 ? '' : 's'}`}
        </Button>
        <span className="text-xs text-muted-foreground">
          {hechas}/{pares.length} listas
        </span>
        {progreso && (
          <span className="text-xs">
            {progreso.detalle ??
              `Generando ${progreso.hechas + 1} de ${progreso.total}${
                progreso.actual !== null ? ` (foto ${progreso.actual + 1})` : ''
              }…`}
          </span>
        )}
      </div>

      {progreso?.detalle && (
        <p role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {progreso.detalle}
        </p>
      )}

      {pares.length === 0 && (
        <p className="text-sm text-muted-foreground">El inmueble no tiene fotos todavía.</p>
      )}

      <ul className="grid gap-3 sm:grid-cols-2">
        {pares.map(({ orden, original, variante }) => (
          <li key={orden} className="overflow-hidden rounded-md border">
            <div className="grid grid-cols-2 gap-px bg-border">
              <figure className="bg-background">
                <img src={original} alt={`Foto ${orden + 1}`} className="aspect-[4/3] w-full object-cover" loading="lazy" />
                <figcaption className="px-2 py-1 text-xs text-muted-foreground">Original</figcaption>
              </figure>
              <figure className="bg-background">
                {variante ? (
                  <img
                    src={variante.url}
                    alt={`Variante ${orden + 1}`}
                    className="aspect-[4/3] w-full object-cover"
                    loading="lazy"
                  />
                ) : (
                  <div className="flex aspect-[4/3] items-center justify-center bg-muted/50 text-xs text-muted-foreground">
                    Sin variante
                  </div>
                )}
                <figcaption className="px-2 py-1 text-xs text-muted-foreground">Variante</figcaption>
              </figure>
            </div>
            {variante && (
              <div className="flex items-center justify-between border-t px-2 py-1">
                <span className="text-xs text-muted-foreground">Foto {orden + 1}</span>
                <Button size="sm" variant="ghost" className="text-destructive" disabled={ocupado} onClick={() => alBorrar(orden)}>
                  <Trash2 /> Quitar variante
                </Button>
              </div>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
