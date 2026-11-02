/* [28AE-1] Aviso de solicitudes nuevas de producción. Fuera de la bandeja el
 * contador se consulta al montar y cada minuto; al abrir la bandeja todas se
 * marcan como vistas y el aviso baja a cero. */
import { useEffect, useState } from 'react';
import { contarSolicitudesNoVistas, marcarSolicitudesVistas } from '../../data/solicitudes/api';

const CADA_MS = 60_000;

export function useSolicitudesNoVistas(enBandeja: boolean): number {
  const [total, setTotal] = useState(0);

  useEffect(() => {
    let vigente = true;
    async function consultar() {
      try {
        if (enBandeja) {
          await marcarSolicitudesVistas();
          if (vigente) setTotal(0);
        } else {
          const n = await contarSolicitudesNoVistas();
          if (vigente) setTotal(n);
        }
      } catch {
        // Aviso secundario: si falla, se conserva el último valor. El fallo real
        // lo muestra la bandeja de Solicitudes al abrirla (no depende de este aviso).
      }
    }
    void consultar();
    // Dentro de la bandeja no hace falta vigilar: al salir se vuelve a consultar.
    if (enBandeja) {
      return () => {
        vigente = false;
      };
    }
    const id = setInterval(() => void consultar(), CADA_MS);
    return () => {
      vigente = false;
      clearInterval(id);
    };
  }, [enBandeja]);

  return total;
}
