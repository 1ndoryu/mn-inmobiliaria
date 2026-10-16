/* [13AA-12 · 14AA-2] Resolver las diferencias de un inmueble con el servidor.
 * Cada campo se decide con «lo último» (por defecto) o «Conservar lo viejo»; el plan y
 * la escritura viven en `useModalDiferenciasSync`. Aplicar escribe en los dos
 * lados (F2): antes de hacerlo pide confirmación y nombra el servidor (D4/D16),
 * y después muestra lo escrito, lo pendiente y lo que sigue distinto. El padre
 * lo monta con `key` por inmueble (estado limpio al abrir). */
import type { Inmueble } from '@/domain/inmueble';
import { nombreClave } from '@/domain/sync/sync-inmueble';
import { useModalDiferenciasSync } from '@/hooks/sync/use-modal-diferencias-sync';
import { ColumnaLado, Lista, describirDecision, describirOrigen, etiquetaColumna } from './piezas-diferencias-sync';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';

interface Props {
  /* `null` = cerrado. */
  slug: string | null;
  titulo: string;
  revision: number;
  /* Inmueble local: la fila de la receta se dibuja con sus fotos. */
  inmueble: Inmueble;
  alCerrar: () => void;
}

export function ModalDiferenciasSync({ slug, titulo, revision, inmueble, alCerrar }: Props) {
  const {
    plan,
    error,
    etiqueta,
    actualizando,
    conFotos,
    puedeAplicar,
    conservarViejo,
    alElegir,
    limpiarEleccion,
    destino,
    informe,
    ejecutando,
    errorAplicar,
    confirmando,
    pedirConfirmacion,
    cancelarConfirmacion,
    aplicar,
  } = useModalDiferenciasSync(slug, revision);

  return (
    <Dialog open={slug !== null} onOpenChange={(abierto) => !abierto && alCerrar()}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>Resolver diferencias con el servidor</DialogTitle>
          <DialogDescription>{titulo || 'Sin título'}</DialogDescription>
        </DialogHeader>
        {error && (
          <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</p>
        )}
        {!plan && !error && <p className="text-sm text-muted-foreground">Calculando el plan…</p>}
        {plan && etiqueta && (
          <div className="space-y-3">
            <div className="flex flex-wrap items-center gap-2 text-sm">
              <Badge variant="secondary" className={etiqueta.clase}>
                {etiqueta.texto}
              </Badge>
              <span>
                Cambios en Local: <strong>{plan.en_local}</strong> · en Servidor: <strong>{plan.en_servidor}</strong>
              </span>
              {plan.estimado && (
                <span className="text-xs text-muted-foreground">
                  Sin base previa: la dirección se estima por la última edición de cada lado.
                </span>
              )}
              {conservarViejo.length > 0 && (
                <Button type="button" variant="outline" size="sm" className="ml-auto" onClick={limpiarEleccion}>
                  Usar lo último en todo
                </Button>
              )}
            </div>
            <p className="text-sm text-muted-foreground">
              <strong className="text-foreground">Lo último:</strong> queda el valor del lado que cambió; si cambiaron los dos o
              no hay base previa, el del inmueble editado más recientemente.{' '}
              <strong className="text-foreground">Conservar lo viejo:</strong> queda el valor del otro lado, solo en ese campo.
            </p>
            {plan.filas.length === 0 ? (
              <p className="text-sm text-muted-foreground">Sin diferencias de contenido.</p>
            ) : (
              <ul className={`space-y-2 transition-opacity ${actualizando ? 'opacity-60' : ''}`} aria-busy={actualizando}>
                {plan.filas.map((fila) => {
                  const viejo = conservarViejo.includes(fila.clave);
                  const nombre = nombreClave(fila.clave);
                  return (
                    <li key={fila.clave} className="space-y-2 rounded-lg border p-3">
                      <div className="flex flex-wrap items-start justify-between gap-2">
                        <div>
                          <p className="font-medium">{nombre}</p>
                          <p className="text-xs text-muted-foreground">{describirOrigen(fila.lado)}</p>
                        </div>
                        <div role="group" aria-label={`Valor que queda en ${nombre}`} className="flex gap-1.5">
                          <Button
                            type="button"
                            size="sm"
                            variant={viejo ? 'outline' : 'default'}
                            aria-pressed={!viejo}
                            onClick={() => alElegir(fila.clave, false)}
                          >
                            Lo último
                          </Button>
                          <Button
                            type="button"
                            size="sm"
                            variant={viejo ? 'default' : 'outline'}
                            aria-pressed={viejo}
                            onClick={() => alElegir(fila.clave, true)}
                          >
                            Conservar lo viejo
                          </Button>
                        </div>
                      </div>
                      <p className="text-sm">{describirDecision(fila, viejo)}</p>
                      <div className="grid grid-cols-2 gap-3">
                        <ColumnaLado
                          nombre="Local"
                          clave={fila.clave}
                          valor={fila.local}
                          miniatura={fila.miniatura_local}
                          inmueble={inmueble}
                          etiqueta={etiquetaColumna(fila, 'local')}
                          queda={fila.elegido === 'local'}
                        />
                        <ColumnaLado
                          nombre="Servidor"
                          clave={fila.clave}
                          valor={fila.servidor}
                          miniatura={fila.miniatura_servidor}
                          inmueble={inmueble}
                          etiqueta={etiquetaColumna(fila, 'servidor')}
                          queda={fila.elegido === 'servidor'}
                        />
                      </div>
                      {fila.clave === 'receta' && plan.filas.some((f) => f.clave.startsWith('foto:')) && (
                        <p className="text-xs text-muted-foreground">
                          Las dos vistas se dibujan con las fotos de Local. Como las fotos también difieren, el servidor puede
                          estar numerando otras.
                        </p>
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
            {actualizando && <p className="text-xs text-muted-foreground">Actualizando el plan…</p>}
            {informe ? (
              <div className="space-y-3 border-t pt-3">
                <p className="text-sm font-medium">Cambios aplicados</p>
                <Lista titulo="Escrito en Local" pasos={informe.en_local} />
                <Lista titulo="Escrito en el Servidor" pasos={informe.en_servidor} />
                {informe.pendientes.length > 0 && (
                  <>
                    <Lista titulo="Queda pendiente" pasos={informe.pendientes} />
                    <p className="text-xs text-muted-foreground">
                      Las fotos se aplican en la fase siguiente (F3); la receta, solo cuando el lado que gana la tiene.
                    </p>
                  </>
                )}
                {informe.residuo.length > 0 && (
                  <>
                    <Lista titulo="Sigue distinto" pasos={informe.residuo} />
                    <p className="text-xs text-muted-foreground">Vuelve a abrir el diálogo para resolverlo.</p>
                  </>
                )}
                {informe.fallo && (
                  <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                    Quedó a medias: {informe.fallo}. Lo ya escrito no se deshace.
                  </p>
                )}
                <div className="flex justify-end">
                  <Button type="button" onClick={alCerrar}>
                    Cerrar
                  </Button>
                </div>
              </div>
            ) : (
              <div className="flex flex-wrap items-center justify-end gap-3 border-t pt-3">
                {confirmando ? (
                  /* El plan es largo: al abrirse, la confirmación se trae a la vista
                   * para que sus botones no queden por debajo del pliegue. */
                  <div
                    ref={(nodo) => nodo && requestAnimationFrame(() => nodo.scrollIntoView({ block: 'end' }))}
                    className="w-full space-y-2 rounded-md border border-destructive/40 bg-destructive/5 p-3"
                  >
                    <p className="text-sm font-medium">¿Aplicar los cambios?</p>
                    <p className="text-sm">
                      Se escribe en los dos lados: <strong>{plan.en_local}</strong> en Local y{' '}
                      <strong>{plan.en_servidor}</strong> en el Servidor:{' '}
                      <span className="font-mono font-medium">{destino?.host}</span>
                    </p>
                    {conFotos && (
                      <p className="text-xs text-muted-foreground">
                        Las fotos no se aplican todavía (fase F3): quedarán pendientes y el informe lo dirá.
                      </p>
                    )}
                    <p className="text-xs text-muted-foreground">Lo ya escrito no se deshace.</p>
                    {errorAplicar && (
                      <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                        {errorAplicar}
                      </p>
                    )}
                    <div className="flex justify-end gap-2">
                      <Button type="button" variant="outline" onClick={cancelarConfirmacion} disabled={ejecutando}>
                        Cancelar
                      </Button>
                      <Button type="button" onClick={() => void aplicar()} disabled={ejecutando}>
                        {ejecutando ? 'Escribiendo…' : 'Aplicar ahora'}
                      </Button>
                    </div>
                  </div>
                ) : (
                  <>
                    <p className="mr-auto text-xs text-muted-foreground">
                      {destino === null
                        ? 'No se pudo leer el servidor: no se puede aplicar.'
                        : 'Aplicar escribe en Local y en el Servidor; se pide confirmación.'}
                    </p>
                    <Button type="button" disabled={!puedeAplicar} onClick={pedirConfirmacion}>
                      Aplicar cambios
                    </Button>
                  </>
                )}
              </div>
            )}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
