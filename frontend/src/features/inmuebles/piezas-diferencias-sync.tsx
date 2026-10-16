/* [16AA-2] Piezas de dibujo de «Resolver diferencias con el servidor»: una
 * columna por lado, la vista previa de la receta, las listas del informe y las
 * frases que describen cada fila. Separadas del diálogo, que solo decide. */
import { useMemo } from 'react';
import type { EleccionSync, FilaPlanSync } from '@/data/inmuebles/sync';
import { fotosVisiblesDe, type Inmueble } from '@/domain/inmueble';
import { recetaDesdeCanonica, resumenReceta, resolverReceta } from '@/domain/plantilla-publicidad';
import { nombreClave } from '@/domain/sync/sync-inmueble';
import { VistaPrevia } from '@/features/publicidad/vista-publicidad';

const NOMBRE_LADO: Record<EleccionSync, string> = { local: 'Local', servidor: 'Servidor' };

function contraria(eleccion: EleccionSync): EleccionSync {
  return eleccion === 'local' ? 'servidor' : 'local';
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

/* Por qué decide el lado que decide: quién cambió, o el criterio si no hay un único cambio. */
export function describirOrigen(lado: FilaPlanSync['lado']): string {
  if (lado === 'local') return 'Cambió en Local';
  if (lado === 'servidor') return 'Cambió en Servidor';
  if (lado === 'ambos') return 'Cambiaron los dos: decide la última edición del inmueble';
  return 'Sin base previa: decide la última edición del inmueble';
}

/* Frase que resume la fila: qué queda y qué se escribe en el otro lado. */
export function describirDecision(fila: FilaPlanSync, viejo: boolean): string {
  const queda = NOMBRE_LADO[fila.elegido];
  const otro = NOMBRE_LADO[contraria(fila.elegido)];
  const criterio = viejo ? 'Conservas lo viejo' : 'Se usa lo último';
  return fila.accion === 'escribir'
    ? `${criterio}: el valor de ${queda}. ${otro} se actualizará con él.`
    : `${criterio}: en ${queda} no existe. Se quitará de ${otro}.`;
}

/* Qué pasa con cada lado: el que queda, o el que se reemplaza, se añade o se quita. */
export function etiquetaColumna(fila: FilaPlanSync, lado: EleccionSync): string {
  if (lado === fila.elegido) return 'Se queda';
  if (fila.accion === 'quitar') return 'Se quita';
  const valor = lado === 'local' ? fila.local : fila.servidor;
  return valor === null ? 'Se añade' : 'Se reemplaza';
}

/* [16AA-1] La receta se guarda como JSON (`{"fondo_idx":1,…}`): ilegible. Se
 * dibuja la vista previa real —la misma de la página Publicidad— y se resume
 * en palabras; el JSON queda plegado por si hace falta el detalle. */
function VistaReceta({ inmueble, valor }: { inmueble: Inmueble; valor: string | null }) {
  const receta = useMemo(() => recetaDesdeCanonica(valor), [valor]);
  const comp = useMemo(() => (receta ? resolverReceta(inmueble, receta) : null), [inmueble, receta]);
  const sinFotos = fotosVisiblesDe(inmueble).length === 0;
  return (
    <>
      {comp && !sinFotos ? (
        <VistaPrevia inmueble={inmueble} comp={comp} ancho={320} />
      ) : (
        <div className="flex h-40 items-center justify-center rounded-md bg-muted px-2 text-center text-xs text-muted-foreground">
          {sinFotos ? 'El inmueble no tiene fotos' : 'Sin receta propia'}
        </div>
      )}
      <p className="text-xs text-muted-foreground">{resumenReceta(receta)}</p>
      {valor ? (
        <details className="text-xs text-muted-foreground">
          <summary className="cursor-pointer select-none">Ver el JSON</summary>
          <pre className="mt-1 max-h-24 overflow-auto whitespace-pre-wrap break-all">{valor}</pre>
        </details>
      ) : null}
    </>
  );
}

/* Una columna por lado. En las fotos, la miniatura va sobre el texto para comparar a la vista. */
export function ColumnaLado({
  nombre,
  clave,
  valor,
  miniatura,
  inmueble,
  etiqueta,
  queda,
}: {
  nombre: string;
  clave: string;
  valor: string | null;
  miniatura: string | null;
  inmueble: Inmueble;
  etiqueta: string;
  queda: boolean;
}) {
  const esFoto = clave.startsWith('foto:');
  const esReceta = clave === 'receta';
  return (
    <div
      className={`min-w-0 space-y-1.5 rounded-md p-2 ${queda ? 'bg-primary/5 ring-1 ring-primary/40' : 'border border-dashed'}`}
    >
      <div className="flex items-center justify-between gap-2">
        <p className="text-xs font-medium text-muted-foreground">{nombre}</p>
        <span className={`text-xs font-semibold ${queda ? 'text-primary' : 'text-muted-foreground'}`}>{etiqueta}</span>
      </div>
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
      {esReceta && valor !== null ? (
        <VistaReceta inmueble={inmueble} valor={valor} />
      ) : (
        <p
          title={valor ?? undefined}
          className={`max-h-32 overflow-auto whitespace-pre-wrap break-words text-sm ${valor === null ? 'italic text-muted-foreground' : ''}`}
        >
          {describirValor(clave, valor)}
        </p>
      )}
    </div>
  );
}

/* Qué quedó tras aplicar: lo escrito en cada lado, lo pendiente y el residuo. */
export function Lista({ titulo, pasos }: { titulo: string; pasos: string[] }) {
  if (pasos.length === 0) return null;
  return (
    <>
      <p className="text-sm font-medium">{titulo}</p>
      <ul className="list-disc space-y-1 pl-5 text-sm">
        {pasos.map((paso) => (
          <li key={paso}>{paso}</li>
        ))}
      </ul>
    </>
  );
}
