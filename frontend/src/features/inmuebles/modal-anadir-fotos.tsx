import { CircleAlert, CircleCheck, ImagePlus, Loader2, X } from 'lucide-react';
import type { Inmueble } from '@/domain/inmueble';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { cn } from '@/lib/utils';
import { useModalAnadirFotos } from '@/hooks/inmuebles/use-modal-anadir-fotos';

interface Props {
  inmueble: Inmueble | null;
  alCambiarAbierto: (abierto: boolean) => void;
  onConfirmar: (nuevas: string[]) => void;
}

/* Puesta en escena antes de subir: se eligen ficheros, se comprimen y se
 * clasifican en "se subirán" / "ya subidas (se omiten)" / "no legibles".
 * Nada se guarda hasta pulsar Subir; Cancelar lo descarta todo. */
export function ModalAnadirFotos({ inmueble, alCambiarAbierto, onConfirmar }: Props) {
  const {
    nuevas,
    omitidas,
    errores,
    procesando,
    arrastrando,
    setArrastrando,
    inputFotos,
    elegir,
    quitarNueva,
  } = useModalAnadirFotos(inmueble);

  const confirmar = () => {
    if (nuevas.length === 0 || procesando) return;
    onConfirmar(nuevas);
  };

  return (
    <Dialog open={inmueble !== null} onOpenChange={alCambiarAbierto}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ImagePlus className="h-4 w-4" /> Añadir fotos
          </DialogTitle>
          <DialogDescription>
            {inmueble ? (
              <>
                Elige todas las fotos de «{inmueble.titulo || 'Sin título'}»: las que ya están subidas se
                detectan y se omiten. Nada se guarda hasta que pulses Subir.
              </>
            ) : (
              'Elige fotos para añadir al inmueble.'
            )}
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-4">
          <div className="space-y-1">
            <div
              role="button"
              tabIndex={0}
              aria-label="Elegir fotos para añadir"
              onClick={() => inputFotos.current?.click()}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') inputFotos.current?.click();
              }}
              onDragOver={(e) => {
                e.preventDefault();
                setArrastrando(true);
              }}
              onDragLeave={() => setArrastrando(false)}
              onDrop={(e) => {
                e.preventDefault();
                setArrastrando(false);
                void elegir(e.dataTransfer.files);
              }}
              className={
                'flex min-h-24 cursor-pointer flex-col items-center justify-center gap-1 rounded-md border border-dashed p-4 text-sm text-muted-foreground ' +
                (arrastrando ? 'border-primary bg-accent' : 'border-input')
              }
            >
              {procesando ? (
                <Loader2 className="h-5 w-5 animate-spin" />
              ) : (
                <ImagePlus className="h-5 w-5" />
              )}
              {procesando ? 'Comprimiendo…' : 'Suelta fotos aquí o pulsa para elegirlas'}
            </div>
            <input
              ref={inputFotos}
              type="file"
              accept="image/*"
              multiple
              className="hidden"
              onChange={(e) => void elegir(e.target.files)}
            />
          </div>

          {nuevas.length > 0 && (
            <div className="space-y-1">
              <p className="flex items-center gap-1 text-sm font-medium">
                <CircleCheck className="h-4 w-4 text-emerald-600" /> Se subirán ({nuevas.length})
              </p>
              <div className="grid grid-cols-4 gap-2 sm:grid-cols-6">
                {nuevas.map((f, idx) => (
                  <div key={f.slice(-32) + idx} className="group relative aspect-square overflow-hidden rounded-md border">
                    <img src={f} alt={`Nueva foto ${idx + 1}`} className="h-full w-full object-cover" />
                      <button
                        type="button"
                        aria-label={`Quitar nueva foto ${idx + 1}`}
                        onClick={() => quitarNueva(idx)}
                      className="absolute right-1 top-1 rounded bg-black/60 p-1 text-white opacity-0 transition group-hover:opacity-100"
                    >
                      <X className="h-3 w-3" />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}

          {omitidas.length > 0 && (
            <div className="space-y-1">
              <p className="text-sm font-medium text-muted-foreground">
                Ya subidas — se omitirán ({omitidas.length})
              </p>
              <div className="grid grid-cols-6 gap-2 sm:grid-cols-8">
                {omitidas.map((f, idx) => (
                  <div
                    key={f.slice(-32) + idx}
                    className="aspect-square overflow-hidden rounded-md border opacity-60"
                    title="Ya está subida"
                  >
                    <img src={f} alt="" className="h-full w-full object-cover" />
                  </div>
                ))}
              </div>
            </div>
          )}

          {errores.length > 0 && (
            <p role="alert" className="rounded-md border border-amber-300 bg-amber-50 p-3 text-sm text-amber-900">
              <CircleAlert className="mr-1 inline h-4 w-4" />
              No se pudieron leer ({errores.length}): {errores.join(' ')}
            </p>
          )}
        </div>

        <DialogFooter>
          <Button type="button" variant="ghost" onClick={() => alCambiarAbierto(false)}>
            Cancelar
          </Button>
          <Button type="button" onClick={confirmar} disabled={nuevas.length === 0 || procesando} className={cn(procesando && 'opacity-70')}>
            {procesando ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" /> Comprimiendo…
              </>
            ) : (
              <>
                <ImagePlus className="h-4 w-4" /> Subir {nuevas.length > 0 ? `${nuevas.length} foto${nuevas.length === 1 ? '' : 's'}` : 'fotos'}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
