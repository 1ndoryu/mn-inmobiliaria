import { Download, EllipsisVertical, Loader2, RotateCcw, Undo2, X } from 'lucide-react';
import type { FotoMejora } from '@/domain/foto-mejora';
import type { VarianteServidor } from '@/domain/inmueble';
import type { InfoReintento } from '@/hooks/mejora/use-cola-mejora';
import { Badge } from '@/components/ui/badge';
import { Button, ButtonPlano } from '@/components/ui/button';
/* [08AA-32] Ampliación, restauración y descarga viven en useTarjetaFotoMejora. */
import { useTarjetaFotoMejora, type ImagenAmpliada } from '@/hooks/imagenes/use-tarjeta-foto-mejora';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

/* Título de la imagen ampliada; el `variante-N` se lee como «Versión N». */
function tituloAmpliada(cual: ImagenAmpliada | null): string {
  if (cual === 'mejorada') return 'Foto mejorada';
  if (cual?.startsWith('variante-')) return `Versión ${cual.slice('variante-'.length)}`;
  return 'Foto original';
}

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

/* Tarjeta compacta: original, mejorada y —si existen— las variantes v2, v3…
 * en columnas, para comparar; el clic en cada una la abre completa. La mejorada se guarda en
 * la mejor resolución que devuelva el backend. `reintento` muestra el
 * reintento programado por el backend (F19).
 * F21: Cancelar saca la foto de la cola (o aborta el proceso) y la deja
 * pendiente limpia; Reintentar la mueve (los errores finales reutilizan el
 * original retenido sin re-subir).
 * 259A-1: IDs de fila del original y la mejorada visibles + Restaurar
 * (borra la mejorada del servidor con confirmación; la copia local queda
 * pendiente para mejorarla de nuevo).
 * 259A-3: acciones (Reintentar/Cancelar/Restaurar) en menú contextual de
 * 3 puntos + Descargar original/mejorada (vía blob, ver descarga.ts).
 * 30AA-4: columna por variante (v2, v3…) cuando existen; la rejilla se
 * reparte el ancho entre las columnas presentes. */
export function TarjetaFotoMejora(props: {
  foto: FotoMejora;
  /** ID de fila del original en la API (null en borradores sin subir). */
  idOriginal: string | null;
  /** ID de fila de la mejorada en la API (null si aún no hay). */
  idServidor: string | null;
  /* [30AA-4] Variantes generadas de esta foto, ordenadas por versión. */
  variantes?: VarianteServidor[];
  alReintentar: (foto: FotoMejora) => void;
  alCancelar: (foto: FotoMejora) => void;
  /** Restaura el original (borra la mejorada del servidor); null = sin mejorada que restaurar. */
  alRestaurar: (() => Promise<void>) | null;
  ocupado: boolean;
  reintento?: InfoReintento | null;
}) {
  const {
    foto,
    idOriginal,
    idServidor,
    variantes = [],
    alReintentar,
    alCancelar,
    alRestaurar,
    ocupado,
    reintento,
  } = props;
  /* [08AA-32] Estado interactivo en el hook; aquí JSX + presentación. */
  const {
    ampliada,
    setAmpliada,
    restaura,
    descarga,
    conReintento,
    urlAmpliada,
    puedeRestaurar,
    restaurar,
    descargar,
  } = useTarjetaFotoMejora({ foto, alRestaurar, idServidor, variantes, reintento });

  /* Una columna por imagen disponible: original y mejorada siempre, más una
   * por variante. `flex` reparte el ancho; con tres o más, cada una es más
   * estrecha pero siguen alineadas. */
  const columnas = 2 + variantes.length;

  return (
    <div className="overflow-hidden rounded-lg border bg-card">
      <div
        className="grid gap-px bg-border"
        style={{ gridTemplateColumns: `repeat(${columnas}, minmax(0, 1fr))` }}
      >
        <div className="bg-card">
          <ButtonPlano
            type="button"
            onClick={() => setAmpliada('original')}
            title="Ver original completa"
            className="block w-full cursor-zoom-in"
          >
            <img src={foto.original} alt="Original" className="aspect-square w-full object-cover" loading="lazy" />
          </ButtonPlano>
          <p className="px-2 pt-1 text-[11px] text-muted-foreground">Original</p>
          {idOriginal && (
            <p
              className="px-2 pb-1 font-mono text-[10px] break-all text-muted-foreground"
              title={`ID del original en el servidor: ${idOriginal}`}
            >
              ID {idOriginal}
            </p>
          )}
        </div>
        <div className="bg-card">
          {foto.mejorada ? (
            <ButtonPlano
              type="button"
              onClick={() => setAmpliada('mejorada')}
              title="Ver mejorada completa"
              className="block w-full cursor-zoom-in"
            >
              <img src={foto.mejorada} alt="Mejorada" className="aspect-square w-full object-cover" loading="lazy" />
            </ButtonPlano>
          ) : (
            <div className="flex aspect-square w-full items-center justify-center bg-muted text-xs text-muted-foreground">
              {foto.estado === 'procesando' ? <Loader2 className="h-5 w-5 animate-spin" /> : 'Sin mejorar'}
            </div>
          )}
          <p className="px-2 pt-1 text-[11px] text-muted-foreground">Mejorada</p>
          {idServidor && (
            <p
              className="px-2 pb-1 font-mono text-[10px] break-all text-muted-foreground"
              title={`ID de la mejorada en el servidor: ${idServidor}`}
            >
              ID {idServidor}
            </p>
          )}
        </div>
        {/* 30AA-4: una columna por versión generada (v2, v3…). Solo aparecen
          las que existen: sin variantes la tarjeta queda como antes. */}
        {variantes.map((v) => (
          <div key={v.id} className="bg-card">
            <ButtonPlano
              type="button"
              onClick={() => setAmpliada(`variante-${v.version}`)}
              title={`Ver versión ${v.version} completa`}
              className="block w-full cursor-zoom-in"
            >
              <img
                src={v.url}
                alt={`Versión ${v.version}`}
                className="aspect-square w-full object-cover"
                loading="lazy"
              />
            </ButtonPlano>
            <p className="px-2 pt-1 text-[11px] text-muted-foreground">Versión {v.version}</p>
            <p
              className="px-2 pb-1 font-mono text-[10px] break-all text-muted-foreground"
              title={`ID de la variante en el servidor: ${v.id}`}
            >
              ID {v.id}
            </p>
          </div>
        ))}
      </div>
      <div className="flex items-center gap-2 px-2 py-2">
        <Badge variant={variante(foto.estado)}>{ETIQUETA[foto.estado]}</Badge>
        {foto.intentos > 0 && !conReintento && (
          <span className="text-[11px] text-muted-foreground">intento {foto.intentos}</span>
        )}
        {conReintento && (
          <span className="text-[11px] text-muted-foreground" title={conReintento.motivo ?? undefined}>
            {(conReintento.enSeg ?? 0) > 60
              ? `reintento en ~${Math.ceil((conReintento.enSeg ?? 0) / 60)} min`
              : 'reintento en cola'}{' '}
            · intento {conReintento.intentos}
          </span>
        )}
        {/* 259A-3: acciones en menú contextual de 3 puntos (mismo patrón
          que tabla-inmuebles): Reintentar/Cancelar/Restaurar + descargas. */}
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button
                variant="ghost"
                size="icon"
                className="ml-auto h-7 w-7"
                title="Acciones de la foto"
                aria-label="Acciones de la foto"
              >
                <EllipsisVertical />
              </Button>
            }
          />
          <DropdownMenuContent align="end" className="w-56">
            <DropdownMenuItem
              disabled={ocupado || foto.estado === 'procesando'}
              onClick={() => alReintentar(foto)}
            >
              <RotateCcw /> Reintentar
            </DropdownMenuItem>
            {foto.estado === 'procesando' && (
              <DropdownMenuItem onClick={() => alCancelar(foto)}>
                <X /> Cancelar
              </DropdownMenuItem>
            )}
            <DropdownMenuSeparator />
            <DropdownMenuItem disabled={descarga.enCurso} onClick={() => void descargar('original')}>
              <Download /> Descargar original
            </DropdownMenuItem>
            {foto.mejorada && (
              <DropdownMenuItem disabled={descarga.enCurso} onClick={() => void descargar('mejorada')}>
                <Download /> Descargar mejorada
              </DropdownMenuItem>
            )}
            {variantes.map((v) => (
              <DropdownMenuItem
                key={v.id}
                disabled={descarga.enCurso}
                onClick={() => void descargar(`variante-${v.version}`)}
              >
                <Download /> Descargar versión {v.version}
              </DropdownMenuItem>
            ))}
            {puedeRestaurar && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuItem disabled={ocupado || restaura.enCurso} onClick={() => void restaurar()}>
                  {restaura.enCurso ? <Loader2 className="animate-spin" /> : <Undo2 />} Restaurar
                </DropdownMenuItem>
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      {foto.error && <p className="px-2 pb-2 text-[11px] text-destructive">{foto.error}</p>}
      {restaura.error && <p className="px-2 pb-2 text-[11px] text-destructive">{restaura.error}</p>}
      {descarga.error && <p className="px-2 pb-2 text-[11px] text-destructive">{descarga.error}</p>}
      <Dialog open={ampliada !== null} onOpenChange={(abierto) => !abierto && setAmpliada(null)}>
        <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-3xl">
          <DialogHeader>
            <DialogTitle>{tituloAmpliada(ampliada)}</DialogTitle>
          </DialogHeader>
          <div className="overflow-hidden rounded-lg border">
            <img src={urlAmpliada} alt={tituloAmpliada(ampliada)} className="h-auto w-full" />
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
