import { useCallback, useEffect, useState } from 'react';
import { avisarAlCerrar, fijarRuta, rutaActual, suscribirEvento, temporizar } from '../../../platform/ventana';
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
/* [25AA-10] Ruta compartible: abrir `/publicar` arranca con el modal abierto. */
const RUTA_PUBLICAR = '/publicar';
const adapter = new LocalStorageAdapter();

type FaseEnvio = 'reposo' | 'subiendo' | 'enviando' | 'enviada';

export function useModalPublicar() {
  const [abierto, setAbierto] = useState(() => rutaActual() === RUTA_PUBLICAR);
  // [25AA-10] Cerrar con borrador pide confirmación; el borrador queda en localStorage.
  const [confirmandoCierre, setConfirmandoCierre] = useState(false);
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

  // [25AA-10] La URL sigue al modal: `/publicar` mientras está abierto, `/` al cerrar.
  useEffect(() => {
    const enRuta = rutaActual() === RUTA_PUBLICAR;
    if (abierto && !enRuta) fijarRuta(RUTA_PUBLICAR);
    // Al cerrar se reemplaza (sin entrada nueva): Atrás no reabre el modal.
    if (!abierto && enRuta) history.replaceState(null, '', '/');
  }, [abierto]);

  // Atrás del navegador fuera de `/publicar` cierra el modal sin preguntar (ya salió de la ruta).
  useEffect(() => {
    const alVolver = () => {
      if (rutaActual() !== RUTA_PUBLICAR) {
        setConfirmandoCierre(false);
        setAbierto(false);
      }
    };
    return suscribirEvento('popstate', alVolver);
  }, []);

  const abrir = useCallback(() => {
    setEnvio({ fase: 'reposo', error: null });
    setConfirmandoCierre(false);
    setAbierto(true);
  }, []);

  const cerrar = useCallback(() => {
    setConfirmandoCierre(false);
    setAbierto(false);
  }, []);

  // Clic fuera o Escape: si hay algo escrito, pide confirmación antes de cerrar.
  const pedirCierre = useCallback(() => {
    if (envio.fase !== 'enviada' && borradorSolicitudTieneContenido(borrador)) setConfirmandoCierre(true);
    else cerrar();
  }, [borrador, envio.fase, cerrar]);

  const seguirEditando = useCallback(() => setConfirmandoCierre(false), []);

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

  return {
    abierto,
    abrir,
    cerrar,
    pedirCierre,
    confirmandoCierre,
    seguirEditando,
    borrador,
    actualizar,
    agregarFotos,
    quitarFoto,
    enviar,
    envio,
  };
}
