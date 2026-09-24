import { useState } from 'react';

/* [249A-5] Pestillo de montaje para overlays `lazy` (PSI escritorio: los 5
 * chunks iban en la ruta crítica porque se montaban cerrados y `lazy`
 * descarga al montar). Montan en la primera apertura y quedan montados:
 * la descarga sale de la carga inicial y la animación de cierre de Radix
 * (`data-closed:animate-out`) se conserva. Ajuste durante el render
 * (patrón doc. React para estado derivado): sin efecto, sin re-render
 * en cascada. */
export function useMontarAlAbrir(abierto: boolean): boolean {
  const [visto, setVisto] = useState(abierto);
  if (abierto && !visto) setVisto(true);
  return visto;
}
