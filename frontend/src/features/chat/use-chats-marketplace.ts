// Estado del panel por chat (07AA-7): lista de hilos + detalle del elegido.
// El hilo es la clave de ventana del puente (trae nombre+aviso: solo-admin).
// [09AA-31] La lista se pagina por scroll: `lista` crece con cada página.
// [10AA-4] `soloHuerfanos` pide al backend solo los hilos sin ficha conocida.

import { useCallback, useEffect, useRef, useState } from 'react';
import {
  archivarChat,
  borrarBorradorChat,
  borrarChat,
  leerChat,
  listarChats,
  type ChatFila,
  type ChatResumen,
} from '../../data/chat/marketplace-chats';
import { ErrorApi } from '../../data/inmuebles/api';

export interface DetalleChat {
  hilo: string;
  filas: ChatFila[];
  cargando: boolean;
}

/* [09AA-31] Filas por página (el backend admite 1–100). */
const LIMITE = 25;

function mensajeError(e: unknown): string {
  return e instanceof ErrorApi ? e.message : 'Fallo inesperado.';
}

/* [10AA-4] `soloHuerfanos`: la lista y su paginación se limitan a esos hilos. */
export function useChatsMarketplace(soloHuerfanos: boolean) {
  const [lista, setLista] = useState<ChatResumen[]>([]);
  const [total, setTotal] = useState(0);
  const [hayMas, setHayMas] = useState(false);
  const [cargando, setCargando] = useState(false);
  const [seleccion, setSeleccion] = useState<DetalleChat | null>(null);
  const [error, setError] = useState<string | null>(null);
  /* Guarda síncrona: el sentinel puede pedir dos páginas antes del re-render. */
  const enCurso = useRef(false);
  /* [10AA-4] Número de cada petición: al cambiar de filtro, la respuesta de la
   * consulta anterior que llegue tarde se descarta. */
  const generacion = useRef(0);

  /* Trae una página. `reemplazar`: la lista pasa a ser esta página; si no, se
   * añade tras la última fila sin duplicar hilos. Un reemplazo siempre se lanza
   * (cambio de pestaña o recarga); la paginación espera a la petición en curso. */
  const cargar = useCallback(
    async (reemplazar: boolean, cursor?: ChatResumen) => {
      if (!reemplazar && enCurso.current) return;
      const esta = ++generacion.current;
      enCurso.current = true;
      setCargando(true);
      try {
        const p = await listarChats({ limite: LIMITE, cursor, soloHuerfanos });
        if (esta !== generacion.current) return;
        setLista((actual) => {
          if (reemplazar) return p.chats;
          const vistos = new Set(actual.map((c) => c.thread_id));
          return [...actual, ...p.chats.filter((c) => !vistos.has(c.thread_id))];
        });
        setTotal(p.total);
        setHayMas(p.hay_mas);
        setError(null);
      } catch (e: unknown) {
        if (esta === generacion.current) setError(mensajeError(e));
      } finally {
        if (esta === generacion.current) {
          enCurso.current = false;
          setCargando(false);
        }
      }
    },
    [soloHuerfanos],
  );

  const recargar = useCallback(() => cargar(true), [cargar]);

  /* Siguiente página: cursor = última fila cargada. */
  const cargarMas = useCallback(() => {
    const cursor = lista[lista.length - 1];
    if (!hayMas || !cursor) return;
    void cargar(false, cursor);
  }, [lista, hayMas, cargar]);

  useEffect(() => {
    void recargar();
  }, [recargar]);

  const elegir = useCallback(async (hilo: string) => {
    setSeleccion({ hilo, filas: [], cargando: true });
    try {
      const filas = await leerChat(hilo);
      setSeleccion({ hilo, filas, cargando: false });
    } catch (e: unknown) {
      setError(mensajeError(e));
      setSeleccion(null);
    }
  }, []);

  /* [09AA-30] Acciones del menú de tres puntos. Archivar y borrar quitan el hilo
   * de la lista sin recargar, para no perder las páginas ya cargadas; borrar
   * borrador recarga, porque el hilo puede seguir con sus correcciones. Si el
   * hilo tocado era el elegido, se quita el detalle: evita mostrar filas que ya
   * no existen; el usuario vuelve a elegirlo si lo quiere ver. */
  const actuar = useCallback(
    async (hilo: string, accion: () => Promise<unknown>, quitar: boolean) => {
      try {
        await accion();
        if (quitar) {
          setLista((actual) => actual.filter((c) => c.thread_id !== hilo));
          setTotal((t) => Math.max(0, t - 1));
        } else {
          await recargar();
        }
        setError(null);
        setSeleccion((s) => (s?.hilo === hilo ? null : s));
      } catch (e: unknown) {
        setError(mensajeError(e));
      }
    },
    [recargar],
  );

  const archivar = useCallback((hilo: string) => actuar(hilo, () => archivarChat(hilo), true), [actuar]);
  const borrar = useCallback((hilo: string) => actuar(hilo, () => borrarChat(hilo), true), [actuar]);
  const borrarBorrador = useCallback(
    (hilo: string) => actuar(hilo, () => borrarBorradorChat(hilo), false),
    [actuar],
  );

  return {
    lista,
    total,
    hayMas,
    cargando,
    seleccion,
    error,
    recargar,
    cargarMas,
    elegir,
    archivar,
    borrar,
    borrarBorrador,
  };
}
