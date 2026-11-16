/* [30AA-9] Modal exclusivo de vínculos Marketplace de UNA ficha: se abre al
 * pulsar el badge de la columna Vínculo («falta v2», «falta v1», «sin
 * vincular»…), así se pone el ID de cada versión en cada cuenta sin pasar por
 * el modal de edición completo.
 *
 * Reutiliza `useVersionado` (mismas llamadas y misma recarga tras cada
 * mutación) y `SeccionFacebook` (un campo por versión y cuenta), de modo que
 * la conducta y el aspecto son idénticos a la pestaña Facebook del modal de
 * inmueble. Al cerrar avisa con la ficha releída del servidor: la fila de la
 * tabla repinta su badge con los avisos que quedan. */
import { useEffect, useRef } from 'react';
import { Link2 } from 'lucide-react';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { leerRemoto } from '@/data/inmuebles/api';
import type { Inmueble } from '@/domain/inmueble';
import { versionesDe } from '@/domain/versiones';
import { useVersionado } from '@/hooks/inmuebles/use-versionado';
import { SeccionFacebook } from './versionado/seccion-facebook';

interface Props {
  inmueble: Inmueble | null;
  alCerrar: () => void;
  /** La ficha tal como quedó en el servidor, para reponer la fila. */
  alActualizar: (inmueble: Inmueble) => void;
}

export function ModalVinculos({ inmueble, alCerrar, alActualizar }: Props) {
  const abierto = inmueble !== null;
  /* Sin fotos ni prompt: este modal solo toca cuentas y avisos, pero el hook
   * pide ambos para la generación de variantes. */
  const versionado = useVersionado({
    inmuebleId: inmueble?.id ?? null,
    fotosParaVariante: [],
    promptVariante: '',
  });
  const { recargar, datos, cuentas, vinculos, ocupado, aviso, error, vincular, desvincular, crear } = versionado;

  /* Se recuerda el id abierto para releerlo al cerrar aunque el padre ya
   * haya vaciado la selección. */
  const abiertoId = useRef<string | null>(null);
  useEffect(() => {
    if (abierto) abiertoId.current = inmueble?.id ?? null;
  }, [abierto, inmueble?.id]);

  async function cerrar() {
    const id = abiertoId.current;
    abiertoId.current = null;
    alCerrar();
    if (!id) return;
    /* El badge se calcula en el servidor (`faltanAvisos`): se relee la ficha
     * para que la fila lo pinte al día. Si falla, la fila se queda como
     * estaba; no se rompe el cierre por eso. */
    try {
      alActualizar(await leerRemoto(id));
    } catch {
      /* Sin red: la próxima recarga de la lista lo corrige. */
    }
  }

  /* Se avisa también a cada mutación, para que el badge no espere al cierre. */
  async function conReposicion(tarea: () => Promise<void>) {
    await tarea();
    const id = abiertoId.current;
    if (!id) return;
    try {
      alActualizar(await leerRemoto(id));
    } catch {
      /* Igual que al cerrar: se corrige en la siguiente carga. */
    }
  }

  const versiones = datos ? versionesDe(datos) : [1];

  return (
    <Dialog open={abierto} onOpenChange={(siguiente) => !siguiente && void cerrar()}>
      {/* `grid-cols-1` explícito: la columna implícita del `grid` del diálogo
       * crece con el contenido y el formulario se salía por la derecha. */}
      <DialogContent className="grid-cols-1 sm:max-w-xl">
        <DialogHeader>
          <DialogTitle className="flex min-w-0 items-start gap-2">
            <Link2 className="mt-0.5 h-4 w-4 shrink-0" />
            {/* Dos líneas como mucho: el título cabe entero en vez de quedar
             * cortado a media palabra. */}
            <span className="line-clamp-2">Vínculos de Marketplace: {inmueble?.titulo || 'Sin título'}</span>
          </DialogTitle>
          <DialogDescription>
            Un aviso por versión y cuenta: pon el ID de Marketplace de cada una. Al cerrar, la fila se actualiza.
          </DialogDescription>
        </DialogHeader>

        {error ? (
          <p role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </p>
        ) : null}
        {aviso && !error ? (
          <p role="alert" className="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm">
            {aviso}
          </p>
        ) : null}

        {!datos ? (
          <p className="text-sm text-muted-foreground">Cargando versiones…</p>
        ) : (
          <SeccionFacebook
            cuentas={cuentas}
            vinculos={vinculos}
            versiones={versiones}
            ocupado={ocupado}
            compacto
            alVincular={(cuentaId, version, marketplaceId) =>
              void conReposicion(() => vincular(cuentaId, version, marketplaceId))
            }
            alDesvincular={(cuentaId, version) => void conReposicion(() => desvincular(cuentaId, version))}
            alCrearCuenta={(nombre, url) =>
              void conReposicion(async () => {
                await crear(nombre, url);
              })
            }
          />
        )}

        {/* Recarga explícita: útil tras publicar en Facebook con otra pestaña. */}
        {datos && (
          <p className="text-right">
            <button
              type="button"
              className="text-xs text-muted-foreground underline-offset-4 hover:underline"
              disabled={ocupado}
              onClick={() => void recargar()}
            >
              Recargar
            </button>
          </p>
        )}
      </DialogContent>
    </Dialog>
  );
}
