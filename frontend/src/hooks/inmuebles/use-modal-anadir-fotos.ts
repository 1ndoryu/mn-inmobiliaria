import { useEffect, useRef, useState } from 'react';
import { ficherosADataUrls } from '@/data/inmuebles/imagenes';
import type { Inmueble } from '@/domain/inmueble';

/* Bandeja de puesta en escena (3 useState): `{nuevas, omitidas, errores}` |
 * `procesando` | `arrastrando`. Nada se guarda hasta pulsar Subir. */

/* Se eligen ficheros, se comprimen y se clasifican en "se subirán" /
 * "ya subidas (se omiten)" / "no legibles". */
interface Bandeja {
  nuevas: string[];
  omitidas: string[];
  errores: string[];
}

export function useModalAnadirFotos(inmueble: Inmueble | null) {
  const [bandeja, setBandeja] = useState<Bandeja>({ nuevas: [], omitidas: [], errores: [] });
  const [procesando, setProcesando] = useState(false);
  const [arrastrando, setArrastrando] = useState(false);
  const inputFotos = useRef<HTMLInputElement>(null);

  // Cada inmueble empieza con la bandeja vacía.
  useEffect(() => {
    setBandeja({ nuevas: [], omitidas: [], errores: [] });
    setProcesando(false);
    setArrastrando(false);
    if (inputFotos.current) inputFotos.current.value = '';
  }, [inmueble?.id]);

  const elegir = async (files: FileList | File[] | null) => {
    if (!files || files.length === 0 || !inmueble || procesando) return;
    setProcesando(true);
    try {
      const { urls, errores: fallos } = await ficherosADataUrls(Array.from(files));
      const base = [...inmueble.fotos, ...bandeja.nuevas];
      const frescas: string[] = [];
      const repetidas: string[] = [];
      for (const u of urls) {
        if (base.includes(u) || frescas.includes(u)) repetidas.push(u);
        else frescas.push(u);
      }
      if (frescas.length > 0)
        setBandeja((b) => ({ ...b, nuevas: [...b.nuevas, ...frescas.filter((u) => !b.nuevas.includes(u))] }));
      if (repetidas.length > 0)
        setBandeja((b) => ({ ...b, omitidas: [...b.omitidas, ...repetidas.filter((u) => !b.omitidas.includes(u))] }));
      if (fallos.length > 0) setBandeja((b) => ({ ...b, errores: [...b.errores, ...fallos] }));
    } finally {
      setProcesando(false);
      if (inputFotos.current) inputFotos.current.value = '';
    }
  };

  const quitarNueva = (idx: number) =>
    setBandeja((b) => ({ ...b, nuevas: b.nuevas.filter((_, j) => j !== idx) }));

  return {
    nuevas: bandeja.nuevas,
    omitidas: bandeja.omitidas,
    errores: bandeja.errores,
    procesando,
    arrastrando,
    setArrastrando,
    inputFotos,
    elegir,
    quitarNueva,
  };
}
