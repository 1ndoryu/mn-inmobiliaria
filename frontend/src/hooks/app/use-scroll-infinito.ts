// Scroll infinito de una lista (09AA-31): `raizRef` va en la caja con scroll y
// `centinelaRef` al final de la lista. Al verse el centinela llama a `alVerFin`.
// Solo observa si `activo`. Quien pasa `alVerFin` debe darle identidad nueva
// cuando cambian los datos (p. ej. `useCallback` con la lista): así se vuelve a
// observar tras cada carga, por si el centinela sigue a la vista.

import { useEffect, useRef } from 'react';

export function useScrollInfinito(activo: boolean, alVerFin: () => void) {
  const raizRef = useRef<HTMLDivElement>(null);
  const centinelaRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const nodo = centinelaRef.current;
    if (!nodo || !activo) return;
    const observador = new IntersectionObserver(
      (entradas) => {
        if (entradas.some((e) => e.isIntersecting)) alVerFin();
      },
      { root: raizRef.current, rootMargin: '0px 0px 80px 0px' },
    );
    observador.observe(nodo);
    return () => observador.disconnect();
  }, [activo, alVerFin]);

  return { raizRef, centinelaRef };
}
