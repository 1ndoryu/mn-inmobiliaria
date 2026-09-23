import { useCallback, useState } from 'react';
import type { ConfigCopy } from '../../domain/copy';
import { guardarConfigCopy, leerConfigCopy } from '../../data/ia/repositorio-copy';

// Configuración del Copy para redes: vive en localStorage con valores
// seguros por defecto (igual que la config de mejora en `use-fotos-mejora`).
export function useConfigCopy() {
  const [config, setConfig] = useState<ConfigCopy>(() => leerConfigCopy());

  const guardar = useCallback((nueva: ConfigCopy) => {
    guardarConfigCopy(nueva);
    setConfig({ ...nueva });
  }, []);

  return { config, guardar };
}
