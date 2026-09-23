import type { ReactNode } from 'react';

/* Piezas compartidas de los formularios de inmueble: etiqueta y estilo de
 * selects. Extraído de `modal-inmueble` (Sentinel limite-lineas). */

export const CLASE_SELECT =
  'flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm capitalize focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30';

export function Etiqueta(props: { children: ReactNode; error?: string }) {
  return <span className="block text-sm font-medium">{props.children}</span>;
}
