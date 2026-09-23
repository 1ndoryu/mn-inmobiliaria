import { useEffect, useRef, useState } from 'react';
import { probarConexion, reiniciarCola } from '@/data/mejora/cliente-mejora';
import { copiarTexto } from '@/platform/documento';
import { confirmar, temporizar } from '@/platform/ventana';

/* Acciones de la tira de diagnóstico (3 useState):
 * `copiado` | prueba `{probando, texto}` | `reiniciando`.
 * El portapapeles y los temporizadores viven en `platform/*`, nunca sueltos. */

export function useTiraDiagnostico(alReiniciarTodo: () => Promise<number>, refrescar: () => void) {
  const [copiado, setCopiado] = useState(false);
  const [prueba, setPrueba] = useState<{ probando: boolean; texto: string | null }>({
    probando: false,
    texto: null,
  });
  const [reiniciando, setReiniciando] = useState(false);
  const cancelarAviso = useRef<(() => void) | null>(null);

  // Si el componente se desmonta, no se deja el temporizador colgado.
  useEffect(
    () => () => {
      cancelarAviso.current?.();
    },
    [],
  );

  /* Deja un JSON listo para pegar al asistente. */
  const copiarDiagnostico = async (diagnostico: unknown) => {
    const ok = await copiarTexto(JSON.stringify(diagnostico, null, 2));
    if (!ok) return;
    setCopiado(true);
    cancelarAviso.current?.();
    cancelarAviso.current = temporizar(2000, () => setCopiado(false));
  };

  /* Valida las cookies con login real y mensaje de texto, sin gastar foto
   * (OK solo si sirven para imágenes). */
  const probar = async () => {
    setPrueba({ probando: true, texto: null });
    let texto: string | null = null;
    try {
      const r = await probarConexion();
      texto =
        r === null
          ? 'Sin contacto con el backend.'
          : r.ok
            ? 'Cookies OK: Gemini autentica texto e imágenes.'
            : `Fallan las cookies: ${r.error ?? ''}`.slice(0, 200);
    } finally {
      setPrueba({ probando: false, texto });
      refrescar();
    }
  };

  /* Encolado, en proceso y error vuelven a empezar; lo lista no se toca. */
  const reiniciar = async () => {
    if (
      !confirmar(
        '¿Reiniciar la cola? Lo encolado, en proceso y en error vuelve a pendiente con intentos a cero. Lo lista no se toca.',
      )
    ) {
      return;
    }
    setReiniciando(true);
    try {
      const r = await reiniciarCola();
      const movidas = await alReiniciarTodo();
      setPrueba({
        probando: false,
        texto: r.ok
          ? `Cola reiniciada: ${movidas} fotos a pendiente${r.reencolados > 0 ? `, ${r.reencolados} reencolados en backend` : ''}${r.descartados > 0 ? `, ${r.descartados} duplicados fuera` : ''}.`
          : `Frontal reiniciado (${movidas} a pendiente), pero el backend no contestó.`,
      });
    } finally {
      setReiniciando(false);
      refrescar();
    }
  };

  return {
    copiado,
    probando: prueba.probando,
    prueba: prueba.texto,
    reiniciando,
    copiarDiagnostico,
    probar,
    reiniciar,
  };
}
