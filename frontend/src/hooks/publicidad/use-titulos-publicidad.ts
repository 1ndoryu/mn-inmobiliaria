import { useState } from 'react';

/* Texto tecleado en los dos títulos de la imagen publicitaria. `null` = sin
 * tocar: vale el efectivo (automático o personalizado de la receta). */
export function useTitulosPublicidad() {
  const [texto1, setTexto1] = useState<string | null>(null);
  const [texto2, setTexto2] = useState<string | null>(null);

  const cambiarTitulo = (campo: 'texto1' | 'texto2', valor: string) => {
    if (campo === 'texto1') setTexto1(valor);
    else setTexto2(valor);
  };

  const reiniciar = () => {
    setTexto1(null);
    setTexto2(null);
  };

  return { texto1, texto2, cambiarTitulo, reiniciar };
}
