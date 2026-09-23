import { useCallback, useEffect, useState } from 'react';
import { avisarAlCerrar, temporizar } from '../../platform/ventana';
import { CLAVE_BORRADOR, LocalStorageAdapter } from '../../data/sesion/storage';
import { idbBorrarFotos, idbGuardarFotos, idbLeerFotos } from '../../data/mejora/idb';
import { DRAFT_VACIO, draftTieneContenido, type InmuebleDraft } from '../../domain/inmueble';

const adapter = new LocalStorageAdapter();
// Clave de las fotos del borrador en el almacén `fotos-inmueble` (no colisiona
// con ids de inmueble, que son UUIDs). Los metadatos siguen en localStorage.
const CLAVE_FOTOS_BORRADOR = 'borrador';

// Borrador del modal con autosave: sobrevive a cierres accidentales,
// recargas y apagados. Solo se limpia con guardado o descarte explícito.
// Las fotos viven en IndexedDB (sin límite); localStorage solo metadatos.
export function useBorrador() {
  const [borrador, setBorrador] = useState<InmuebleDraft>(DRAFT_VACIO);
  const [hayBorradorGuardado, setHayBorradorGuardado] = useState(false);
  const [cargado, setCargado] = useState(false);

  // Lee el borrador persistido una sola vez al montar, migrando a IndexedDB
  // las fotos que aún estén en localStorage (registros anteriores).
  useEffect(() => {
    let vivo = true;
    (async () => {
      const meta = adapter.leer<InmuebleDraft>(CLAVE_BORRADOR);
      let fotos: string[] = [];
      try {
        if (meta && Array.isArray(meta.fotos) && meta.fotos.length > 0) {
          await idbGuardarFotos(CLAVE_FOTOS_BORRADOR, meta.fotos);
          fotos = [...meta.fotos];
          adapter.escribir(CLAVE_BORRADOR, { ...meta, fotos: [] });
        } else {
          fotos = (await idbLeerFotos(CLAVE_FOTOS_BORRADOR)) ?? [];
        }
      } catch {
        // Sin IndexedDB: se usa lo que haya en localStorage, aunque quepa poco.
        fotos = meta && Array.isArray(meta.fotos) ? meta.fotos : [];
      }
      if (!vivo) return;
      if (meta && draftTieneContenido({ ...meta, fotos })) {
        setBorrador({ ...DRAFT_VACIO, ...meta, fotos });
        setHayBorradorGuardado(true);
      }
      setCargado(true);
    })();
    return () => {
      vivo = false;
    };
  }, []);

  // Autosave con debounce: cada cambio se persiste a los 300ms.
  useEffect(() => {
    if (!cargado) return;
    if (!draftTieneContenido(borrador)) return;
    return temporizar(300, () => {
      void (async () => {
        try {
          await idbGuardarFotos(CLAVE_FOTOS_BORRADOR, borrador.fotos);
          adapter.escribir(CLAVE_BORRADOR, { ...borrador, fotos: [] });
          setHayBorradorGuardado(true);
        } catch {
          // Sin IndexedDB: cadena de reserva en localStorage (cabe poco).
          try {
            adapter.escribir(CLAVE_BORRADOR, borrador);
            setHayBorradorGuardado(true);
          } catch {
            try {
              adapter.escribir(CLAVE_BORRADOR, { ...borrador, fotos: [] });
              setHayBorradorGuardado(true);
            } catch {
              // Último recurso: no se puede persistir; el borrador en memoria sigue vivo.
            }
          }
        }
      })();
    });
  }, [borrador, cargado]);

  // Aviso nativo al recargar/cerrar la pestaña con cambios sin guardar.
  useEffect(() => avisarAlCerrar(() => draftTieneContenido(borrador)), [borrador]);

  const restaurarBorrador = useCallback(async () => {
    const meta = adapter.leer<InmuebleDraft>(CLAVE_BORRADOR);
    let fotos: string[] = [];
    try {
      fotos = (await idbLeerFotos(CLAVE_FOTOS_BORRADOR)) ?? [];
    } catch {
      fotos = meta && Array.isArray(meta.fotos) ? meta.fotos : [];
    }
    if (meta) setBorrador({ ...DRAFT_VACIO, ...meta, fotos });
    setHayBorradorGuardado(false);
  }, []);

  const limpiarClaves = useCallback(async () => {
    setBorrador(DRAFT_VACIO);
    try {
      localStorage.removeItem(CLAVE_BORRADOR);
    } catch {
      // Sin almacenamiento: el borrador en memoria ya se limpió.
    }
    try {
      await idbBorrarFotos(CLAVE_FOTOS_BORRADOR);
    } catch {
      // Sin IndexedDB no hay fotos que limpiar.
    }
    setHayBorradorGuardado(false);
  }, []);

  const empezarDeCero = useCallback(async () => {
    await limpiarClaves();
  }, [limpiarClaves]);

  const limpiarTrasGuardar = useCallback(async () => {
    await limpiarClaves();
  }, [limpiarClaves]);

  return {
    borrador,
    setBorrador,
    hayBorradorGuardado,
    cargado,
    restaurarBorrador,
    empezarDeCero,
    limpiarTrasGuardar,
  };
}
