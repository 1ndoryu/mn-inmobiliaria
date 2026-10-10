// Panel por chat del asistente Marketplace (07AA-7): una fila por hilo
// (conversación + borradores + usos + vigencia) y detalle con la foto de
// la conversación (`excerpt_texto`) al lado del texto guardado.

import { usePestanaPersistida } from '../../../hooks/app/use-pestana-persistida';
import { useScrollInfinito } from '../../../hooks/app/use-scroll-infinito';
import { useChatsMarketplace } from './use-chats-marketplace';
import type { ChatFila, ChatResumen } from '../../../data/chat/marketplace-chats';
import { HilosHuerfanos } from './hilos-huerfanos';
import { LogsMarketplace } from './logs-marketplace';
import { VinculoInmueble } from './vinculo-inmueble';
import { Badge } from '@/components/ui/badge';
import { Button, ButtonPlano } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { confirmar } from '@/platform/ventana';
import { EllipsisVertical } from 'lucide-react';

const TABS_MARKETPLACE = ['chats', 'logs', 'huerfanos'] as const;

function fechaCorta(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString('es-VE', { dateStyle: 'short', timeStyle: 'short' });
}

/* Origen de la fila: `ia` | `releer` | null (fila anterior, origen desconocido). */
function textoOrigen(origen: string | null): string {
  if (origen === 'ia') return 'IA';
  if (origen === 'releer') return 'solo foto (releer)';
  return 'origen desconocido';
}

/* Con caché servida: «servido desde caché N veces»; sin usos, «0 usos». */
function textoUsos(usos: number): string {
  if (usos > 0) return `servido desde caché ${usos} ${usos === 1 ? 'vez' : 'veces'}`;
  return `${usos} usos`;
}

/* Tokens y tiempo de la generación IA original: solo filas `origen = "ia"`.
 * Un dato null omite su trozo; el total de tokens solo se muestra con ambos. */
function textoMetricas(f: ChatFila): string[] {
  if (f.origen !== 'ia') return [];
  const { tokens_entrada: entrada, tokens_salida: salida, ms } = f.coste;
  const trozos: string[] = [];
  const piezas: string[] = [];
  if (entrada !== null) piezas.push(`entrada ${entrada}`);
  if (salida !== null) piezas.push(`salida ${salida}`);
  if (piezas.length > 0) {
    const total = entrada !== null && salida !== null ? `${entrada + salida} ` : '';
    trozos.push(`${total}tokens (${piezas.join(' / ')})`);
  }
  if (ms !== null) trozos.push(`${(ms / 1000).toFixed(1)} s`);
  return trozos;
}

/* [08AA-31] Lado por marca de texto: lo propio viaja como `Tú:`/`Tu:`/`You:`
 * (backend 08AA-29); lo demás es del cliente (viaja sin etiqueta). Sin esta
 * separación el excerpt se veía pegado en un solo bloque (hilo angelv). */
function esLadoPropio(linea: string): boolean {
  const marca = linea.trim().toLowerCase();
  return marca.startsWith('tú:') || marca.startsWith('tu:') || marca.startsWith('you:');
}

/* [08AA-31] Quita la marca de lado (`Tú: msg` → `msg`): el Badge ya dice el
 * lado, repetirlo en el texto duplica. Solo se usa con `esLadoPropio`. */
function textoSinMarca(linea: string): string {
  const i = linea.indexOf(':');
  const resto = i < 0 ? '' : linea.slice(i + 1).trim();
  return resto === '' ? linea.trim() : resto;
}

/* El hilo es `comprador|aviso` (clave del puente). El título del chat es el
 * comprador; el aviso distingue dos chats del mismo comprador. Sin `|` el hilo
 * es solo el comprador. El hilo entero queda en el `title` del elemento. */
function partirHilo(hilo: string): { comprador: string; aviso: string } {
  const i = hilo.indexOf('|');
  if (i < 0) return { comprador: hilo.trim(), aviso: '' };
  return { comprador: hilo.slice(0, i).trim() || hilo.trim(), aviso: hilo.slice(i + 1).trim() };
}

export function ChatsMarketplace() {
  /* [09AA-27] La sub-pestaña abierta sobrevive a recargas. Va antes del hook de
   * datos: la pestaña Huérfanos pide al backend solo los hilos sin ficha. */
  const [tab, setTab] = usePestanaPersistida<'chats' | 'logs' | 'huerfanos'>(
    'admin:mensajes:marketplace-tab',
    TABS_MARKETPLACE,
    'chats',
  );
  const {
    lista,
    total,
    hayMas,
    cargando,
    seleccion,
    error,
    recargar,
    cargarMas,
    elegir,
    archivar,
    borrar,
    borrarBorrador,
  } = useChatsMarketplace(tab === 'huerfanos');
  /* [09AA-23] Vínculo del hilo con su inmueble (título + `aviso_conocido`).
   * [09AA-28] Se pinta con `VinculoInmueble` (miniatura + título). */
  const vinculoDe = (hilo: string): ChatResumen | undefined => lista.find((c) => c.thread_id === hilo);
  /* [09AA-30] Solo las dos acciones que borran piden confirmación; archivar
   * oculta el hilo de la lista sin tocar datos. */
  const pedirBorrar = (hilo: string) => {
    if (confirmar(`Borrar el chat ${hilo} con sus borradores y correcciones. No se puede deshacer.`)) {
      void borrar(hilo);
    }
  };
  const pedirBorrarBorrador = (hilo: string) => {
    if (confirmar(`Borrar los borradores no corregidos del chat ${hilo}.`)) {
      void borrarBorrador(hilo);
    }
  };
  /* [09AA-5] Tab de Logs: qué hizo el puente (caché/IA/fallback) por cada
   * borrador, para cazar la «plantilla fantasma» sin leer el log de texto.
   * [09AA-19 F7c] Tab de Huérfanos: hilos sin ficha exacta con vínculo
   * manual (reutiliza esta lista + detalle, sin segundo fetch). */
  /* [09AA-31] Scroll infinito de la lista: solo en la pestaña Chats (el centinela
   * no existe en las otras); al volver a ella `tab` lo vuelve a observar.
   * [10AA-4] Huérfanos tiene su propia instancia: con una sola, el observer no se
   * volvería a enganchar al cambiar de pestaña, porque su centinela es otro nodo. */
  const { raizRef, centinelaRef } = useScrollInfinito(tab === 'chats' && hayMas, cargarMas);
  const huerfanosScroll = useScrollInfinito(tab === 'huerfanos' && hayMas, cargarMas);

  return (
    <div className="grid gap-4">
      <div className="flex gap-2">
        <Button variant={tab === 'chats' ? 'default' : 'outline'} size="sm" onClick={() => setTab('chats')}>
          Chats
        </Button>
        <Button variant={tab === 'logs' ? 'default' : 'outline'} size="sm" onClick={() => setTab('logs')}>
          Logs
        </Button>
        <Button variant={tab === 'huerfanos' ? 'default' : 'outline'} size="sm" onClick={() => setTab('huerfanos')}>
          Huérfanos
        </Button>
      </div>
      {tab === 'logs' ? (
        <LogsMarketplace />
      ) : tab === 'huerfanos' ? (
        <>
          {error && (
            <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error}
            </p>
          )}
          <HilosHuerfanos
            hilos={lista}
            total={total}
            seleccion={seleccion}
            alElegir={(hilo) => void elegir(hilo)}
            raizRef={huerfanosScroll.raizRef}
            centinelaRef={huerfanosScroll.centinelaRef}
          />
        </>
      ) : (
    <div className="grid gap-4 md:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
      {error && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive md:col-span-2">
          {error}
        </p>
      )}
      <section>
        <h3 className="mb-2 text-sm font-medium">
          Chats con borradores ({lista.length} de {total})
        </h3>
        <div ref={raizRef} className="max-h-[60vh] overflow-y-auto pr-1">
          <ul className="space-y-2">
            {lista.map((c) => (
              <li
                key={c.thread_id}
                className={`flex items-start gap-1 rounded-md border hover:bg-muted/50 ${
                  seleccion?.hilo === c.thread_id ? 'border-primary' : ''
                }`}
              >
                <ButtonPlano
                  type="button"
                  onClick={() => void elegir(c.thread_id)}
                  className="min-w-0 flex-1 px-3 py-2 text-left text-xs"
                >
                  <span className="block truncate text-sm font-medium" title={c.thread_id}>
                    {partirHilo(c.thread_id).comprador}
                  </span>
                  <span className="mt-1 block empty:hidden">
                    <VinculoInmueble chat={c} />
                  </span>
                  {!c.inmueble_vinculado && partirHilo(c.thread_id).aviso && (
                    <span className="mt-0.5 block truncate text-muted-foreground">{partirHilo(c.thread_id).aviso}</span>
                  )}
                  <span className="mt-0.5 block text-muted-foreground">
                    {c.borradores} borrador{c.borradores === 1 ? '' : 'es'} · {c.usos} uso{c.usos === 1 ? '' : 's'} ·{' '}
                    {fechaCorta(c.ultimo)}
                  </span>
                </ButtonPlano>
                {/* [09AA-30] Tres puntos por conversación, dentro de la caja de la fila:
                 * archivar, borrar borrador o borrar el chat entero. */}
                <DropdownMenu>
                  <DropdownMenuTrigger
                    render={
                      <Button variant="ghost" size="icon" title="Acciones" aria-label={`Acciones del chat ${c.thread_id}`}>
                        <EllipsisVertical />
                      </Button>
                    }
                  />
                  <DropdownMenuContent align="end" className="w-56">
                    <DropdownMenuItem onClick={() => void archivar(c.thread_id)}>Archivar</DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem onClick={() => pedirBorrarBorrador(c.thread_id)}>Borrar borrador</DropdownMenuItem>
                    <DropdownMenuItem variant="destructive" onClick={() => pedirBorrar(c.thread_id)}>
                      Borrar
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </li>
            ))}
            {lista.length === 0 && !cargando && (
              <li className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
                Sin chats todavía. Aparecen cuando el puente genera borradores.
              </li>
            )}
          </ul>
          {/* [09AA-31] Centinela del scroll infinito: al verse, pide la siguiente página. */}
          <div ref={centinelaRef} aria-hidden className="h-px" />
          {cargando && <p className="py-2 text-center text-xs text-muted-foreground">Cargando chats…</p>}
        </div>
        <div className="mt-2 flex flex-wrap gap-2">
          <Button variant="outline" size="sm" onClick={() => void recargar()}>
            Recargar
          </Button>
        </div>
      </section>
      <section>
        <h3 className="mb-2 flex flex-wrap items-center gap-2 text-sm font-medium">
          {seleccion ? (
            <span title={seleccion.hilo}>Chat de {partirHilo(seleccion.hilo).comprador}</span>
          ) : (
            'Elige un chat para ver sus borradores'
          )}
          {seleccion && <VinculoInmueble chat={vinculoDe(seleccion.hilo)} />}
        </h3>
        {seleccion?.cargando && <p className="text-sm text-muted-foreground">Cargando borradores…</p>}
        <ul className="space-y-2">
          {(seleccion?.filas ?? []).map((f, i) => (
            <li key={`${seleccion?.hilo}-${i}`} className="rounded-md border px-3 py-2 text-xs">
              {/* [08AA-5] La conversación se muestra una sola vez: las filas
               * vienen recientes-primero y cada snapshot trae el hilo
               * completo, así que solo la primera pinta su excerpt.
               * [08AA-31] Como chat real: una línea por mensaje con su Badge
               * (`Tú` lo propio, `Cliente` lo de ella/él) en vez del bloque
               * pegado en un solo `span`. */}
              {i === 0 && f.excerpt_texto && (
                <span className="block space-y-1 border-l-2 border-primary/40 pl-2 text-muted-foreground">
                  {f.excerpt_texto.split('\n').map((linea, j) => {
                    const texto = linea.trim();
                    if (texto === '') return null;
                    const propia = esLadoPropio(texto);
                    return (
                      <span key={j} className="flex flex-wrap items-center gap-1">
                        <Badge variant={propia ? 'default' : 'secondary'} className="text-[10px]">
                          {propia ? 'Tú' : 'Cliente'}
                        </Badge>
                        <span className="min-w-0 flex-1 break-words">
                          {propia ? textoSinMarca(texto) : texto}
                        </span>
                      </span>
                    );
                  })}
                </span>
              )}
              <span className="mt-1 block">{f.respuesta}</span>
              <span className="mt-1 flex flex-wrap items-center gap-2 text-muted-foreground">
                {f.corregida && (
                  <Badge variant="secondary" className="text-[10px]">
                    corregida por la dueña
                  </Badge>
                )}
                <span>
                  {[
                    textoUsos(f.usos),
                    `vigente hasta ${fechaCorta(f.valida_hasta)}`,
                    textoOrigen(f.origen),
                    ...textoMetricas(f),
                  ].join(' · ')}
                </span>
              </span>
            </li>
          ))}
        </ul>
        {seleccion && !seleccion.cargando && seleccion.filas.length === 0 && (
          <p className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
            Este hilo ya no tiene borradores vigentes.
          </p>
        )}
      </section>
    </div>
      )}
    </div>
  );
}
