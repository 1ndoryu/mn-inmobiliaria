import { useCallback, useState } from 'react';
import type { Inmueble } from '../../domain/inmueble';

// Añadir fotos a un inmueble existente: confirma la puesta en escena del
// modal re-deduplicando contra el estado actual (nada duplica aunque las
// fotos cambiasen con el modal abierto) y guarda. Extraído de `App`
// (Sentinel limite-lineas).
export function useAnadirFotos(
  inmuebles: Inmueble[],
  actualizar: (inmueble: Inmueble) => Promise<boolean>,
) {
  const [anadiendoId, setAnadiendoId] = useState<string | null>(null);
  const [notaFotos, setNotaFotos] = useState<string | null>(null);

  const confirmar = useCallback(
    async (nuevas: string[]) => {
      setNotaFotos(null);
      const inmueble = anadiendoId ? inmuebles.find((i) => i.id === anadiendoId) : undefined;
      if (!inmueble) {
        setAnadiendoId(null);
        return;
      }
      const unicas = [...new Set(nuevas)];
      const frescas = unicas.filter((u) => !inmueble.fotos.includes(u));
      const omitidas = unicas.length - frescas.length;
      const nombre = inmueble.titulo || 'Sin título';
      if (frescas.length === 0) {
        setAnadiendoId(null);
        if (omitidas > 0) {
          setNotaFotos(
            `${omitidas} foto${omitidas === 1 ? '' : 's'} ya ${omitidas === 1 ? 'estaba subida' : 'estaban subidas'} en "${nombre}": no se subió nada.`,
          );
        }
        return;
      }
      const ok = await actualizar({ ...inmueble, fotos: [...inmueble.fotos, ...frescas] });
      // Si falla, el banner de error ya lo explica; el modal sigue abierto.
      if (!ok) return;
      setAnadiendoId(null);
      const partes = [
        `${frescas.length} foto${frescas.length === 1 ? '' : 's'} añadida${frescas.length === 1 ? '' : 's'} a "${nombre}".`,
      ];
      if (omitidas > 0) {
        partes.push(`${omitidas} ya ${omitidas === 1 ? 'estaba subida y se omitió' : 'estaban subidas y se omitieron'}.`);
      }
      setNotaFotos(partes.join(' '));
    },
    [anadiendoId, inmuebles, actualizar],
  );

  return { anadiendoId, setAnadiendoId, notaFotos, confirmar };
}
