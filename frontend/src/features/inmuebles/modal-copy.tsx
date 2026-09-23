import { useState } from 'react';
import { Check, Copy, Loader2, Megaphone, RefreshCw } from 'lucide-react';
import { formatearFecha, type Inmueble } from '@/domain/inmueble';
import { copiarTexto } from '@/platform/documento';
import { temporizar } from '@/platform/ventana';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

interface Props {
  inmueble: Inmueble | null;
  generando: boolean;
  error: string | null;
  onGenerar: () => void;
  alCambiarAbierto: (abierto: boolean) => void;
}

function BloqueCopy({ titulo, texto, pista }: { titulo: string; texto: string; pista: string }) {
  const [copiado, setCopiado] = useState(false);

  const copiar = async () => {
    // `false` = portapapeles no disponible (permisos): se copia a mano.
    const ok = await copiarTexto(texto);
    setCopiado(ok);
    if (ok) temporizar(2000, () => setCopiado(false));
  };

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-medium">{titulo}</span>
        <Button variant="outline" size="sm" className="h-7 text-xs" onClick={() => void copiar()}>
          {copiado ? (
            <>
              <Check className="h-3.5 w-3.5" /> Copiado
            </>
          ) : (
            <>
              <Copy className="h-3.5 w-3.5" /> Copiar
            </>
          )}
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">{pista}</p>
      <p className="rounded-md border bg-muted/50 p-3 text-sm whitespace-pre-wrap">{texto}</p>
    </div>
  );
}

/* Modal Copy para redes: muestra las 2 descripciones generadas con IA
 * (corta para la imagen, larga para el pie) con botones de copiar y Regenerar.
 * Si el inmueble aún no tiene copy, lo genera al abrirse. */
export function ModalCopy({ inmueble, generando, error, onGenerar, alCambiarAbierto }: Props) {
  const copy = inmueble?.copy ?? null;
  return (
    <Dialog open={inmueble !== null} onOpenChange={alCambiarAbierto}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Megaphone className="h-4 w-4" /> Copy para redes
          </DialogTitle>
          <DialogDescription>
            {inmueble ? (
              <>
                Textos listos para publicar de <strong>{inmueble.titulo || 'Sin título'}</strong>. Se generan una
                vez con IA y se guardan en el inmueble.
              </>
            ) : (
              'Textos listos para publicar.'
            )}
          </DialogDescription>
        </DialogHeader>

        {generando && (
          <p className="flex items-center gap-2 rounded-md border p-3 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" /> Generando copy con IA…
          </p>
        )}
        {error && (
          <div className="grid gap-2 rounded-md border border-destructive/50 bg-destructive/10 p-3 text-sm">
            <p role="alert">{error}</p>
            <div>
              <Button size="sm" onClick={onGenerar}>
                <RefreshCw className="h-3.5 w-3.5" /> Reintentar
              </Button>
            </div>
          </div>
        )}
        {!generando && !error && !copy && (
          <div className="grid gap-2 rounded-md border border-dashed p-4 text-center text-sm text-muted-foreground">
            <p>Este inmueble aún no tiene copy.</p>
            <div className="flex justify-center">
              <Button size="sm" onClick={onGenerar}>
                <Megaphone className="h-3.5 w-3.5" /> Generar ahora
              </Button>
            </div>
          </div>
        )}
        {copy && (
          <div className="grid gap-4">
            <BloqueCopy titulo="Corta — va en la imagen" texto={copy.corta} pista="1-2 líneas con gancho, sobre la foto." />
            <BloqueCopy titulo="Larga — va debajo (pie del post)" texto={copy.larga} pista="Descripción del post, termina con el llamado a la acción." />
            <p className="text-xs text-muted-foreground">
              Generado el {formatearFecha(copy.actualizadaEn)}
              {copy.modelo ? ` · modelo ${copy.modelo}` : ''}.
            </p>
          </div>
        )}

        <DialogFooter>
          <Button variant="ghost" onClick={() => alCambiarAbierto(false)}>
            Cerrar
          </Button>
          {copy && (
            <Button variant="outline" onClick={onGenerar} disabled={generando}>
              {generando ? <Loader2 className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />}
              Regenerar
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
