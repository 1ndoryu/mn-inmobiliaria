/* [16AA-2] Estado de «Resolver diferencias con el servidor»: el plan, la
 * elección por campo y la escritura en los dos lados. El diálogo solo dibuja.
 * El plan lo calcula el backend y es la fuente de verdad: cada elección vuelve
 * a pedirlo, y aplicar exige que siga vigente (D3). */
import { useEffect, useMemo, useState } from 'react';
import {
  aplicarResolucionSync,
  obtenerDestinoSync,
  planificarResolucionSync,
  type DestinoSync,
  type InformeAplicarSync,
  type PlanResolucionSync,
} from '@/data/inmuebles/sync';
import { etiquetaEstado } from '@/domain/sync/sync-inmueble';

interface Carga {
  plan: PlanResolucionSync | null;
  /* Elección con la que se calculó `plan`: si difiere de la actual, el plan está al día. */
  de: string | null;
  error: string | null;
}

interface Escritura {
  /* Se pidió confirmación y todavía no se ha escrito (D4/D16). */
  confirmando: boolean;
  ejecutando: boolean;
  error: string | null;
}

const SIN_CARGA: Carga = { plan: null, de: null, error: null };
const SIN_ESCRITURA: Escritura = { confirmando: false, ejecutando: false, error: null };

export function useModalDiferenciasSync(slug: string | null, revision: number) {
  const [carga, setCarga] = useState<Carga>(SIN_CARGA);
  const [conservarViejo, setConservarViejo] = useState<string[]>([]);
  /* Servidor que se nombra antes de escribir (D16). */
  const [destino, setDestino] = useState<DestinoSync | null>(null);
  const [escritura, setEscritura] = useState<Escritura>(SIN_ESCRITURA);
  const [informe, setInforme] = useState<InformeAplicarSync | null>(null);
  /* Elección como texto estable, para usarla en las dependencias del efecto. */
  const seleccion = useMemo(() => [...conservarViejo].sort().join('\n'), [conservarViejo]);

  useEffect(() => {
    if (slug === null) return;
    let vigente = true;
    obtenerDestinoSync()
      .then((d) => {
        if (vigente) setDestino(d);
      })
      .catch(() => {
        /* Sin destino no se puede aplicar: el botón queda deshabilitado. */
      });
    return () => {
      vigente = false;
    };
  }, [slug]);

  useEffect(() => {
    if (slug === null) return;
    let vigente = true;
    planificarResolucionSync(slug, seleccion ? seleccion.split('\n') : [])
      .then((p) => {
        if (vigente) setCarga({ plan: p, de: seleccion, error: null });
      })
      .catch((e: unknown) => {
        if (!vigente) return;
        setCarga({ plan: null, de: null, error: e instanceof Error ? e.message : 'No se pudo calcular el plan.' });
      });
    return () => {
      vigente = false;
    };
  }, [slug, revision, seleccion]);

  const { plan } = carga;
  const actualizando = plan !== null && carga.de !== seleccion;
  const conFotos = plan?.filas.some((f) => f.clave.startsWith('foto:')) ?? false;
  const operaciones = plan ? plan.en_local + plan.en_servidor : 0;
  const puedeAplicar = plan !== null && !actualizando && operaciones > 0 && destino !== null && informe === null;

  function alElegir(clave: string, conservar: boolean) {
    setConservarViejo((previo) => {
      const sin = previo.filter((c) => c !== clave);
      return conservar ? [...sin, clave] : sin;
    });
  }

  async function aplicar() {
    if (slug === null || plan === null) return;
    setEscritura((e) => ({ ...e, ejecutando: true, error: null }));
    try {
      setInforme(await aplicarResolucionSync(slug, plan.plan_id, conservarViejo));
      setEscritura(SIN_ESCRITURA);
    } catch (e) {
      const motivo = e instanceof Error ? e.message : 'No se pudo aplicar el plan.';
      setEscritura((previo) => ({ ...previo, ejecutando: false, error: motivo }));
    }
  }

  return {
    plan,
    error: carga.error,
    etiqueta: plan ? etiquetaEstado(plan.estado, plan.estimado) : null,
    actualizando,
    conFotos,
    puedeAplicar,
    conservarViejo,
    alElegir,
    limpiarEleccion: () => setConservarViejo([]),
    destino,
    informe,
    ejecutando: escritura.ejecutando,
    errorAplicar: escritura.error,
    confirmando: escritura.confirmando,
    pedirConfirmacion: () => setEscritura((e) => ({ ...e, confirmando: true })),
    cancelarConfirmacion: () => setEscritura((e) => ({ ...e, confirmando: false, error: null })),
    aplicar,
  };
}
