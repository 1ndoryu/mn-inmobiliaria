import { useState } from 'react';
import { Loader2, RotateCcw, X } from 'lucide-react';
import type { FotoMejora } from '@/domain/foto-mejora';
import type { InfoReintento } from '@/hooks/mejora/use-cola-mejora';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

const ETIQUETA: Record<FotoMejora['estado'], string> = {
  pendiente: 'Pendiente',
  procesando: 'Procesando',
  lista: 'Lista',
  error: 'Error',
};

function variante(estado: FotoMejora['estado']): 'default' | 'secondary' | 'destructive' | 'outline' {
  if (estado === 'lista') return 'default';
  if (estado === 'error') return 'destructive';
  if (estado === 'procesando') return 'secondary';
  return 'outline';
}

/* Tarjeta compacta: original a la izquierda y mejorada a la derecha para
 * comparar; el clic en cada una la abre completa. La mejorada se guarda en
 * la mejor resolución que devuelva el backend. `reintento` muestra el
 * reintento programado por el backend (F19).
 * F21: Cancelar saca la foto de la cola (o aborta el proceso) y la deja
 * pendiente limpia; Reintentar la mueve (los errores finales reutilizan el
 * original retenido sin re-subir). */
export function TarjetaFotoMejora(props: {
  foto: FotoMejora;
  alReintentar: (foto: FotoMejora) => void;
  alCancelar: (foto: FotoMejora) => void;
  ocupado: boolean;
  reintento?: InfoReintento | null;
}) {
  const { foto, alReintentar, alCancelar, ocupado, reintento } = props;
  const [ampliada, setAmpliada] = useState<'original' | 'mejorada' | null>(null);
  const conReintento =
    foto.estado === 'procesando' && reintento && (reintento.motivo || (reintento.enSeg ?? 0) > 0);
  const urlAmpliada = ampliada === 'mejorada' ? (foto.mejorada ?? foto.original) : foto.original;
  return (
    <div className="overflow-hidden rounded-lg border bg-card">
      <div className="grid grid-cols-2 gap-px bg-border">
        <div className="bg-card">
          <button
            type="button"
            onClick={() => setAmpliada('original')}
            title="Ver original completa"
            className="block w-full cursor-zoom-in"
          >
            <img src={foto.original} alt="Original" className="aspect-square w-full object-cover" loading="lazy" />
          </button>
          <p className="px-2 py-1 text-[11px] text-muted-foreground">Original</p>
        </div>
        <div className="bg-card">
          {foto.mejorada ? (
            <button
              type="button"
              onClick={() => setAmpliada('mejorada')}
              title="Ver mejorada completa"
              className="block w-full cursor-zoom-in"
            >
              <img src={foto.mejorada} alt="Mejorada" className="aspect-square w-full object-cover" loading="lazy" />
            </button>
          ) : (
            <div className="flex aspect-square w-full items-center justify-center bg-muted text-xs text-muted-foreground">
              {foto.estado === 'procesando' ? <Loader2 className="h-5 w-5 animate-spin" /> : 'Sin mejorar'}
            </div>
          )}
          <p className="px-2 py-1 text-[11px] text-muted-foreground">Mejorada</p>
        </div>
      </div>
      <div className="flex items-center gap-2 px-2 py-2">
        <Badge variant={variante(foto.estado)}>{ETIQUETA[foto.estado]}</Badge>
        {foto.intentos > 0 && !conReintento && (
          <span className="text-[11px] text-muted-foreground">intento {foto.intentos}</span>
        )}
        {conReintento && (
          <span className="text-[11px] text-muted-foreground" title={reintento.motivo ?? undefined}>
            {(reintento.enSeg ?? 0) > 60
              ? `reintento en ~${Math.ceil((reintento.enSeg ?? 0) / 60)} min`
              : 'reintento en cola'}{' '}
            · intento {reintento.intentos}
          </span>
        )}
        <Button
          variant="ghost"
          size="sm"
          className="ml-auto h-7 px-2 text-xs"
          disabled={ocupado || foto.estado === 'procesando'}
          onClick={() => alReintentar(foto)}
          title="Reintentar mejora"
        >
          <RotateCcw className="h-3.5 w-3.5" /> Reintentar
        </Button>
        {foto.estado === 'procesando' && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs"
            onClick={() => alCancelar(foto)}
            title="Cancelar: fuera de la cola, queda pendiente"
          >
            <X className="h-3.5 w-3.5" /> Cancelar
          </Button>
        )}
      </div>
      {foto.error && <p className="px-2 pb-2 text-[11px] text-destructive">{foto.error}</p>}
      <Dialog open={ampliada !== null} onOpenChange={(abierto) => !abierto && setAmpliada(null)}>
        <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-3xl">
          <DialogHeader>
            <DialogTitle>Foto {ampliada === 'mejorada' ? 'mejorada' : 'original'}</DialogTitle>
          </DialogHeader>
          <div className="overflow-hidden rounded-lg border">
            <img
              src={urlAmpliada}
              alt={ampliada === 'mejorada' ? 'Mejorada' : 'Original'}
              className="h-auto w-full"
            />
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
