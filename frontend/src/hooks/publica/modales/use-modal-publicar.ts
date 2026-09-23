import { useCallback, useEffect, useState } from 'react';
import { avisarAlCerrar, temporizar } from '../../../platform/ventana';
import { LocalStorageAdapter } from '../../../data/sesion/storage';
import { crearSolicitud, subirFotoSolicitud } from '../../../data/publicar/api';
import {
  BORRADOR_SOLICITUD_VACIO,
  MAX_FOTOS_SOLICITUD,
  borradorSolicitudTieneContenido,
  validarBorradorSolicitud,
  type BorradorSolicitud,
} from '../../../domain/solicitud';

// [169A-2] Modal "Publicar mi inmueble": el borrador (incluidas las claves
// de fotos ya subidas) se autosalva en localStorage y sobrevive al cierre.
// Las fotos se suben al elegirlas; el alta solo viaja con claves del servidor.
const CLAVE_BORRADOR_SOLICITUD = 'solicitud:borrador';
const adapter = new LocalStorageAdapter();

type FaseEnvio = 'reposo' | 'subiendo' | 'enviando' | 'enviada';

export function useModalPublicar() {
  const [abierto, setAbierto] = useState(false);
  const [borrador, setBorrador] = useState<BorradorSolicitud>(BORRADOR_SOLICITUD_VACIO);
  const [envio, setEnvio] = useState<{ fase: FaseEnvio; error: string | null }>({
    fase: 'reposo',
    error: null,
  });

  useEffect(() => {
    const previo = adapter.leer<BorradorSolicitud>(CLAVE_BORRADOR_SOLICITUD);
    if (previo && borradorSolicitudTieneContenido({ ...BORRADOR_SOLICITUD_VACIO, ...previo }))
      setBorrador({ ...BORRADOR_SOLICITUD_VACIO, ...previo });
    // Solo al montar: el borrador persistido manda sobre el vacío.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!borradorSolicitudTieneContenido(borrador)) return;
    return temporizar(300, () => {
      try {
        adapter.escribir(CLAVE_BORRADOR_SOLICITUD, borrador);
      } catch {
        // Sin almacenamiento el borrador en memoria sigue vivo.
      }
    });
  }, [borrador]);

  useEffect(() => avisarAlCerrar(() => abierto && borradorSolicitudTieneContenido(borrador)), [abierto, borrador]);

  const abrir = useCallback(() => {
    setEnvio({ fase: 'reposo', error: null });
    setAbierto(true);
  }, []);

  const cerrar = useCallback(() => setAbierto(false), []);

  const actualizar = useCallback((campo: keyof BorradorSolicitud, valor: string) => {
    setBorrador((b) => ({ ...b, [campo]: valor }));
  }, []);

  const quitarFoto = useCallback((clave: string) => {
    setBorrador((b) => ({ ...b, fotosClaves: b.fotosClaves.filter((c) => c !== clave) }));
  }, []);

  const agregarFotos = useCallback(
    async (archivos: FileList | File[]) => {
      const lista = [...archivos].slice(0, MAX_FOTOS_SOLICITUD - borrador.fotosClaves.length);
      if (lista.length === 0) return;
      setEnvio({ fase: 'subiendo', error: null });
      try {
        const subidas = await Promise.all(lista.map((f) => subirFotoSolicitud(f, f.name)));
        setBorrador((b) => ({ ...b, fotosClaves: [...b.fotosClaves, ...subidas.map((s) => s.storage_key)] }));
        setEnvio({ fase: 'reposo', error: null });
      } catch (e) {
        setEnvio({ fase: 'reposo', error: e instanceof Error ? e.message : 'No se pudieron subir las fotos.' });
      }
    },
    [borrador.fotosClaves.length],
  );

  const enviar = useCallback(async () => {
    const problema = validarBorradorSolicitud(borrador);
    if (problema) {
      setEnvio({ fase: 'reposo', error: problema });
      return;
    }
    setEnvio({ fase: 'enviando', error: null });
    try {
      await crearSolicitud(borrador);
      try {
        localStorage.removeItem(CLAVE_BORRADOR_SOLICITUD);
      } catch {
        // El borrador en memoria se limpia igual abajo.
      }
      setBorrador(BORRADOR_SOLICITUD_VACIO);
      setEnvio({ fase: 'enviada', error: null });
    } catch (e) {
      setEnvio({ fase: 'reposo', error: e instanceof Error ? e.message : 'No se pudo enviar la solicitud.' });
    }
  }, [borrador]);

  return { abierto, abrir, cerrar, borrador, actualizar, agregarFotos, quitarFoto, enviar, envio };
}
