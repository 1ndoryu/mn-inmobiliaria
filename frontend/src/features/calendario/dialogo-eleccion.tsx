/* [28AA-6] «Publicar hoy»: lista de inmuebles publicables para elegir uno. Va con foto,
 * título y precio, de mayor a menor precio, y la fecha de su última publicación si ya salió. */
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { MiniaturaFoto } from '@/components/ui/miniatura-foto';
import { formatearPrecio, portadaDe, type Inmueble } from '@/domain/inmueble';

interface DialogoEleccionProps {
  /* Ya filtrados y ordenados por precio (el padre decide el orden). */
  inmuebles: Inmueble[];
  /* Última salida por id de inmueble; ausente = nunca publicado. */
  ultimas: Record<string, string> | null;
  ocupado: boolean;
  alElegir: (inmuebleId: string) => void;
  alCerrar: () => void;
}

/* Fecha corta en español, en la zona local del navegador. */
function fechaCorta(iso: string): string {
  return new Date(iso).toLocaleDateString('es', { day: 'numeric', month: 'short', year: 'numeric' });
}

export function DialogoEleccion({ inmuebles, ultimas, ocupado, alElegir, alCerrar }: DialogoEleccionProps) {
  return (
    <Dialog
      open
      onOpenChange={(abierto: boolean) => {
        if (!abierto && !ocupado) alCerrar();
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Elige el inmueble de hoy</DialogTitle>
          <DialogDescription>Se crea el borrador con la fecha de hoy. Ordenados por precio, de mayor a menor.</DialogDescription>
        </DialogHeader>

        {ultimas === null ? (
          <p className="text-sm text-muted-foreground">Cargando…</p>
        ) : inmuebles.length === 0 ? (
          <p className="text-sm text-muted-foreground">No queda ningún inmueble publicable.</p>
        ) : (
          <ul className="max-h-[60vh] divide-y overflow-y-auto">
            {inmuebles.map((i) => {
              const ultima = ultimas[i.id];
              return (
                <li key={i.id}>
                  <button
                    type="button"
                    onClick={() => alElegir(i.id)}
                    disabled={ocupado}
                    className="flex w-full items-center gap-3 py-2 pr-2 text-left hover:bg-muted disabled:opacity-50"
                  >
                    <MiniaturaFoto src={portadaDe(i)} titulo={i.titulo || 'Inmueble'} />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-medium">{i.titulo || 'Sin título'}</span>
                      {ultima && (
                        <span className="block text-xs text-muted-foreground">Última publicación: {fechaCorta(ultima)}</span>
                      )}
                    </span>
                    <span className="shrink-0 text-sm font-semibold">{i.precio > 0 ? formatearPrecio(i.precio) : '—'}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}

        <div className="flex justify-end">
          <Button variant="outline" onClick={alCerrar} disabled={ocupado}>
            Cancelar
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
