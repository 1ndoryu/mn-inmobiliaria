import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

/* [249A-3] `cn` local: el paquete `cn` (drop-in de clsx+tailwind-merge)
 * arrastraba su propio motor compilado (~41 KB raw en el bundle). `clsx` y
 * `tailwind-merge` ya eran dependencias: misma semantica, menos peso.
 * Punto unico de importacion para `components/ui/*` y features. */
export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
