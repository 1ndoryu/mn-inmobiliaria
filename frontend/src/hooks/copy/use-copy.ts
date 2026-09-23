import { useCallback, useEffect, useRef, useState } from 'react';
import type { Inmueble } from '../../domain/inmueble';
import type { ConfigCopy } from '../../domain/copy';
import { generarCopyConIA } from '../../data/ia/copy-ia';

// Generación del Copy con IA + persistencia en el inmueble.
// Se genera UNA vez (al crear o al abrir el modal sin copy) y se guarda;
// Regenerar lo sobrescribe a mano. Si la IA falla, el inmueble queda intacto.
export function useCopy(actualizar: (inmueble: Inmueble) => Promise<boolean>) {
  const [generando, setGenerando] = useState<Record<string, boolean>>({});
  const [errores, setErrores] = useState<Record<string, string | null>>({});
  const abortar = useRef<AbortController | null>(null);

  useEffect(() => () => abortar.current?.abort(), []);

  const generar = useCallback(
    async (inmueble: Inmueble, config: ConfigCopy): Promise<boolean> => {
      setGenerando((p) => ({ ...p, [inmueble.id]: true }));
      setErrores((p) => ({ ...p, [inmueble.id]: null }));
      abortar.current?.abort();
      const ctrl = new AbortController();
      abortar.current = ctrl;
      const r = await generarCopyConIA({ inmueble, config, signal: ctrl.signal });
      abortar.current = null;
      if (!r.ok) {
        setGenerando((p) => ({ ...p, [inmueble.id]: false }));
        setErrores((p) => ({ ...p, [inmueble.id]: r.motivo }));
        return false;
      }
      const ok = await actualizar({
        ...inmueble,
        copy: {
          corta: r.copy.corta,
          larga: r.copy.larga,
          modelo: r.modelo,
          actualizadaEn: new Date().toISOString(),
        },
        updatedAt: new Date().toISOString(),
      });
      setGenerando((p) => ({ ...p, [inmueble.id]: false }));
      if (!ok) {
        setErrores((p) => ({ ...p, [inmueble.id]: 'El copy se generó pero no se pudo guardar.' }));
      }
      return ok;
    },
    [actualizar],
  );

  return { generando, errores, generar };
}
