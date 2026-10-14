/* [13AA-12 · 14AA-2] Resolver las diferencias de un inmueble con el servidor.
 * Cada campo se decide con «Lo último» (por defecto) o «Conservar lo viejo». El
 * plan lo calcula el backend y es la fuente de verdad: cada elección vuelve a
 * pedirlo. Esta fase solo muestra el plan; aplicarlo llega en F2. El padre lo
 * monta con `key` por inmueble (estado limpio al abrir). */
import { useEffect, useMemo, useState } from 'react';
import { planificarResolucionSync, type FilaPlanSync, type PlanResolucionSync } from '@/data/inmuebles/sync';
import { etiquetaEstado, etiquetaLado, nombreClave } from '@/domain/sync/sync-inmueble';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';

interface Props {
  /* `null` = cerrado. */
  slug: string | null;
  titulo: string;
  revision: number;
  alCerrar: () => void;
}

/* Huella de foto: `presente (<8 hex>)`, `presente (sin calcular)`, `presente` o `sin-archivo`. */
function describirFoto(valor: string): string {
  if (valor === 'presente') return 'Presente';
  if (valor === 'sin-archivo') return 'Archivo no encontrado';
  if (valor === 'presente (sin calcular)') return 'Presente (huella pendiente)';
  const huella = /^presente \(([0-9a-f]+)\)$/.exec(valor);
  return huella ? `Presente (huella ${huella[1]})` : valor;
}

function describirValor(clave: string, valor: string | null): string {
  if (valor === null) return 'No existe';
  if (clave.startsWith('foto:')) return describirFoto(valor);
  return valor || '(vacío)';
}

/* Qué queda y qué se escribe en el otro lado. */
function describirCambio(fila: FilaPlanSync): string {
  const queda = fila.elegido === 'local' ? 'Local' : 'Servidor';
  const otro = fila.elegido === 'local' ? 'Servidor' : 'Local';
  return fila.accion === 'escribir'
    ? `Queda el valor de ${queda}. ${otro} se actualizará con él.`
    : `Queda que no exista en ${queda}. Se quitará de ${otro}.`;
}

/* Una columna por lado. En las fotos, la miniatura va sobre el texto para comparar a la vista. */
function ColumnaLado({
  nombre,
  clave,
  valor,
  miniatura,
}: {
  nombre: string;
  clave: string;
  valor: string | null;
  miniatura: string | null;
}) {
  const esFoto = clave.startsWith('foto:');
  return (
    <div className="min-w-0 space-y-1.5">
      <p className="text-xs font-medium text-muted-foreground">{nombre}</p>
      {esFoto &&
        valor !== null &&
        (miniatura ? (
          <img
            src={miniatura}
            alt={`${nombre}: ${nombreClave(clave)}`}
            loading="lazy"
            className="h-28 w-full rounded-md bg-muted object-contain"
          />
        ) : (
          <div className="flex h-28 items-center justify-center rounded-md bg-muted text-xs text-muted-foreground">
            Sin miniatura
          </div>
        ))}
      <p
        title={valor ?? undefined}
        className={`max-h-32 overflow-auto whitespace-pre-wrap break-words text-sm ${valor === null ? 'italic text-muted-foreground' : ''}`}
      >
        {describirValor(clave, valor)}
      </p>
    </div>
  );
}

export function ModalDiferenciasSync({ slug, titulo, revision, alCerrar }: Props) {
  const [plan, setPlan] = useState<PlanResolucionSync | null>(null);
  /* Elección con la que se calculó `plan`: si difiere de la actual, el plan está al día. */
  const [planDe, setPlanDe] = useState<string | null>(null);
  const [conservarViejo, setConservarViejo] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  /* Elección como texto estable, para usarla en las dependencias del efecto. */
  const seleccion = useMemo(() => [...conservarViejo].sort().join('\n'), [conservarViejo]);

  useEffect(() => {
    if (slug === null) return;
    let vigente = true;
    planificarResolucionSync(slug, seleccion ? seleccion.split('\n') : [])
      .then((p) => {
        if (!vigente) return;
        setPlan(p);
        setPlanDe(seleccion);
        setError(null);
      })
      .catch((e: unknown) => {
        if (!vigente) return;
        setPlan(null);
        setError(e instanceof Error ? e.message : 'No se pudo calcular el plan.');
      });
    return () => {
      vigente = false;
    };
  }, [slug, revision, seleccion]);

  function alElegir(clave: string, conservar: boolean) {
    setConservarViejo((previo) => {
      const sin = previo.filter((c) => c !== clave);
      return conservar ? [...sin, clave] : sin;
    });
  }

  const etiqueta = plan ? etiquetaEstado(plan.estado, plan.estimado) : null;
  const actualizando = plan !== null && planDe !== seleccion;

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
                Se cambiará en Local: <strong>{plan.en_local}</strong> · en Servidor: <strong>{plan.en_servidor}</strong>
              </span>
              {plan.estimado && (
                <span className="text-xs text-muted-foreground">
                  Sin base previa: la dirección se estima por la última edición de cada lado.
                </span>
              )}
              {conservarViejo.length > 0 && (
                <Button type="button" variant="outline" size="sm" className="ml-auto" onClick={() => setConservarViejo([])}>
                  Usar lo último en todo
                </Button>
              )}
            </div>
            <p className="text-xs text-muted-foreground">
              Por defecto queda lo último: el lado que cambió, o el más reciente si cambiaron los dos. «Conservar lo viejo»
              deja en ese campo el valor del otro lado.
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
                      <div className="flex flex-wrap items-center justify-between gap-2">
                        <div>
                          <p className="font-medium">{nombre}</p>
                          <p className="text-xs text-muted-foreground">Cambió: {etiquetaLado(fila.lado)}</p>
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
                      <div className="grid grid-cols-2 gap-3">
                        <ColumnaLado nombre="Local" clave={fila.clave} valor={fila.local} miniatura={fila.miniatura_local} />
                        <ColumnaLado
                          nombre="Servidor"
                          clave={fila.clave}
                          valor={fila.servidor}
                          miniatura={fila.miniatura_servidor}
                        />
                      </div>
                      <p className="text-sm">{describirCambio(fila)}</p>
                    </li>
                  );
                })}
              </ul>
            )}
            {actualizando && <p className="text-xs text-muted-foreground">Actualizando el plan…</p>}
            <div className="flex flex-wrap items-center justify-end gap-3 border-t pt-3">
              <p className="mr-auto text-xs text-muted-foreground">
                Aplicar los cambios llega en la siguiente fase; por ahora solo se muestra el plan.
              </p>
              <Button type="button" disabled>
                Aplicar cambios
              </Button>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
