import { useCallback, useEffect, useRef, useState } from 'react';
import type { Inmueble } from '../../domain/inmueble';
import { actualizarInmueble, cargarInmuebles, crearInmueble, eliminarInmueble, publicarInmueble } from '../../data/inmuebles/repositorio-inmuebles';

// Estado de inmuebles sincronizado con la API (única fuente; nada local).
export function useInmuebles() {
  const [inmuebles, setInmuebles] = useState<Inmueble[]>([]);
  const [cargando, setCargando] = useState(true);
  // Con la API como fuente las fotos siempre están "listas" tras cargar;
  // se conserva la puerta por si la carga queda a medias.
  const [fotosListas, setFotosListas] = useState(false);
  const [aviso, setAviso] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Espejo en memoria: base de cada mutación sin releer el almacén
  // (evita pisar un guardado solapado y no re-deserializa las fotos).
  const listaRef = useRef<Inmueble[]>([]);

  const fijar = useCallback((lista: Inmueble[]) => {
    listaRef.current = lista;
    setInmuebles(lista);
  }, []);

  useEffect(() => {
    let vivo = true;
    cargarInmuebles()
      .then(({ lista, fotosOk }) => {
        if (!vivo) return;
        fijar(lista);
        setFotosListas(fotosOk);
        if (!fotosOk) {
          setError('Las fotos no se pudieron leer (almacén no disponible): no se puede guardar hasta recargar.');
        }
      })
      .catch((e) => {
        if (vivo) setError(e instanceof Error ? e.message : 'No se pudieron cargar los inmuebles.');
      })
      .finally(() => {
        if (vivo) setCargando(false);
      });
    return () => {
      vivo = false;
    };
  }, [fijar]);

  const crear = useCallback(
    async (inmueble: Inmueble): Promise<boolean> => {
      setError(null);
      setAviso(null);
      if (!fotosListas) {
        setError('Las fotos no están disponibles: recarga antes de guardar.');
        return false;
      }
      const { lista, resultado } = await crearInmueble(inmueble, listaRef.current);
      if (resultado.ok) {
        fijar(lista);
        return true;
      }
      setError(resultado.motivo);
      return false;
    },
    [fotosListas, fijar],
  );

  const eliminar = useCallback(
    async (id: string): Promise<void> => {
      setError(null);
      if (!fotosListas) {
        setError('Las fotos no están disponibles: recarga antes de eliminar.');
        return;
      }
      const { lista, resultado } = await eliminarInmueble(id, listaRef.current);
      fijar(lista);
      if (!resultado.ok) setAviso(resultado.motivo);
    },
    [fotosListas, fijar],
  );

  const actualizar = useCallback(
    async (inmueble: Inmueble): Promise<boolean> => {
      setError(null);
      if (!fotosListas) {
        setError('Las fotos no están disponibles: recarga antes de guardar.');
        return false;
      }
      const { lista, resultado } = await actualizarInmueble(inmueble, listaRef.current);
      if (resultado.ok) {
        fijar(lista);
        return true;
      }
      setError(resultado.motivo);
      return false;
    },
    [fotosListas, fijar],
  );

  const publicar = useCallback(
    async (id: string, publicado: boolean): Promise<boolean> => {
      setError(null);
      const { lista, resultado } = await publicarInmueble(id, publicado, listaRef.current);
      if (resultado.ok) {
        fijar(lista);
        return true;
      }
      setError(resultado.motivo);
      return false;
    },
    [fijar],
  );

  /* Repone un inmueble ya persistido por otra vía (p. ej. la receta
   * publicitaria, que hace su propio PUT): solo refresca la lista, sin
   * llamar al servidor. */
  const reponer = useCallback(
    (inmueble: Inmueble) => {
      fijar(listaRef.current.map((i) => (i.id === inmueble.id ? inmueble : i)));
    },
    [fijar],
  );

  return {
    inmuebles: [...inmuebles].sort((a, b) => b.createdAt.localeCompare(a.createdAt)),
    total: inmuebles.length,
    cargando,
    crear,
    eliminar,
    actualizar,
    publicar,
    reponer,
    aviso,
    error,
  };
}
