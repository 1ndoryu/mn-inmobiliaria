import { useState } from 'react';
import { Check, Download, Images, Loader2 } from 'lucide-react';
import type { Inmueble } from '@/domain/inmueble';
import { descargarDataUrl, descargarUrl, esperar } from '@/platform/descarga';
import { extensionDeFoto, slugificar } from '@/hooks/inmuebles/use-modal-ver-inmueble';
import { Badge } from '@/components/ui/badge';
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
  alCerrar: () => void;
}

type Ocupado = 'todas' | 'seleccion' | null;

/* URL a descargar de la foto `indice`: la mejorada del servidor cuando
 * existe, si no el original. */
function urlDescargaDe(inmueble: Inmueble, indice: number): string {
  return inmueble.mejoradasServidor.find((m) => m.orden === indice)?.url ?? inmueble.fotos[indice]!;
}

function tieneMejorada(inmueble: Inmueble, indice: number): boolean {
  return inmueble.mejoradasServidor.some((m) => m.orden === indice);
}

/* Modal de descarga de fotos: muestra cada foto con su botón individual,
 * permite seleccionar y descargar todas o solo las seleccionadas. */
export function ModalDescargarFotos({ inmueble, alCerrar }: Props) {
  const [seleccion, setSeleccion] = useState<number[]>([]);
  const [unoDescargando, setUnoDescargando] = useState<number | null>(null);
  const [ocupado, setOcupado] = useState<Ocupado>(null);
  const [fallo, setFallo] = useState<string | null>(null);
  const abierto = inmueble !== null;
  const total = inmueble?.fotos.length ?? 0;

  const alternar = (indice: number) => {
    setSeleccion((previa) =>
      previa.includes(indice) ? previa.filter((i) => i !== indice) : [...previa, indice],
    );
  };

  async function descargarIndices(inmuebleActual: Inmueble, indices: number[]) {
    const base = slugificar(inmuebleActual.titulo);
    for (let n = 0; n < indices.length; n++) {
      const indice = indices[n]!;
      const url = urlDescargaDe(inmuebleActual, indice);
      const variante = tieneMejorada(inmuebleActual, indice) ? 'mejorada' : 'foto';
      const nombre = `${base}-${variante}-${indice + 1}.${extensionDeFoto(url)}`;
      if (url.startsWith('data:')) {
        descargarDataUrl(url, nombre);
      } else {
        await descargarUrl(url, nombre);
      }
      if (n < indices.length - 1) await esperar(600);
    }
  }

  const descargarUna = async (indice: number) => {
    if (!inmueble || ocupado !== null || unoDescargando !== null) return;
    setUnoDescargando(indice);
    setFallo(null);
    try {
      await descargarIndices(inmueble, [indice]);
    } catch (e) {
      setFallo(e instanceof Error ? e.message : 'No se pudo descargar la foto.');
    } finally {
      setUnoDescargando(null);
    }
  };

  const descargarTodas = async () => {
    if (!inmueble || ocupado !== null) return;
    setOcupado('todas');
    setFallo(null);
    try {
      await descargarIndices(
        inmueble,
        inmueble.fotos.map((_, indice) => indice),
      );
    } catch (e) {
      setFallo(e instanceof Error ? e.message : 'No se pudieron descargar las fotos.');
    } finally {
      setOcupado(null);
    }
  };

  const descargarSeleccion = async () => {
    if (!inmueble || ocupado !== null || seleccion.length === 0) return;
    setOcupado('seleccion');
    setFallo(null);
    try {
      await descargarIndices(inmueble, [...seleccion].sort((a, b) => a - b));
    } catch (e) {
      setFallo(e instanceof Error ? e.message : 'No se pudieron descargar las fotos.');
    } finally {
      setOcupado(null);
    }
  };

  const cerrar = (abrir: boolean) => {
    if (!abrir) {
      setSeleccion([]);
      setFallo(null);
      alCerrar();
    }
  };

  return (
    <Dialog open={abierto} onOpenChange={cerrar}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Images className="h-4 w-4" /> Descargar fotos
          </DialogTitle>
          <DialogDescription>
            La versión mejorada cuando existe, si no el original. Toca una foto para seleccionarla.
          </DialogDescription>
        </DialogHeader>
        {inmueble && total > 0 ? (
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
            {inmueble.fotos.map((foto, indice) => {
              const elegida = seleccion.includes(indice);
              const mejorada = tieneMejorada(inmueble, indice);
              return (
                <div key={`${indice}-${foto}`} className="relative overflow-hidden rounded-md border">
                  <button
                    type="button"
                    onClick={() => alternar(indice)}
                    title={elegida ? `Quitar foto ${indice + 1} de la selección` : `Seleccionar foto ${indice + 1}`}
                    aria-pressed={elegida}
                    className={`block w-full outline-none transition focus-visible:ring-2 focus-visible:ring-ring/40 ${
                      elegida ? 'ring-2 ring-primary ring-inset' : ''
                    }`}
                  >
                    <img
                      src={mejorada ? urlDescargaDe(inmueble, indice) : foto}
                      alt={`Foto ${indice + 1}`}
                      loading="lazy"
                      className="h-28 w-full object-cover"
                    />
                  </button>
                  <span className="absolute top-1.5 left-1.5 rounded bg-background/90 px-1.5 py-0.5 text-[11px] font-medium">
                    {indice + 1}
                  </span>
                  {mejorada && (
                    <Badge variant="secondary" className="absolute top-1.5 right-1.5 px-1.5 py-0 text-[11px]">
                      Mejorada
                    </Badge>
                  )}
                  {elegida && (
                    <span className="absolute bottom-1.5 left-1.5 flex h-5 w-5 items-center justify-center rounded-full bg-primary text-primary-foreground">
                      <Check className="h-3.5 w-3.5" />
                    </span>
                  )}
                  <Button
                    variant="secondary"
                    size="icon"
                    className="absolute right-1.5 bottom-1.5 h-7 w-7"
                    title={`Descargar foto ${indice + 1}`}
                    aria-label={`Descargar foto ${indice + 1}`}
                    disabled={ocupado !== null || unoDescargando !== null}
                    onClick={() => void descargarUna(indice)}
                  >
                    {unoDescargando === indice ? (
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    ) : (
                      <Download className="h-3.5 w-3.5" />
                    )}
                  </Button>
                </div>
              );
            })}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">Sin fotos para descargar.</p>
        )}
        {fallo ? <p className="text-xs text-destructive">{fallo}</p> : null}
        <DialogFooter className="items-center">
          <p className="mr-auto text-xs text-muted-foreground">
            {seleccion.length > 0 ? `${seleccion.length} seleccionada${seleccion.length === 1 ? '' : 's'}` : 'Sin selección'}
          </p>
          <Button variant="outline" disabled={ocupado !== null || total === 0} onClick={() => void descargarTodas()}>
            {ocupado === 'todas' ? <Loader2 className="h-4 w-4 animate-spin" /> : <Download className="h-4 w-4" />}
            Descargar todas
          </Button>
          <Button
            disabled={ocupado !== null || seleccion.length === 0}
            onClick={() => void descargarSeleccion()}
          >
            {ocupado === 'seleccion' ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Download className="h-4 w-4" />
            )}
            Descargar seleccionadas{seleccion.length > 0 ? ` (${seleccion.length})` : ''}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
