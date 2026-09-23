import { useCallback, useEffect, useRef, useState } from 'react';
import type { ConfigMejora, FotoMejora, ParcheFoto } from '../../domain/foto-mejora';
import type { EventoMejora, TipoEventoMejora } from '../../domain/historial-mejora';
import {
  borrarFotosDeInmueble,
  guardarConfig,
  importarMejoradasServidor,
  importarOriginales,
  leerConfig,
  purgarHuerfanas,
  purgarMejoradasDuplicadas,
  limpiarHistorial,
  listarFotos,
  listarHistorial,
  marcarEstado,
  registrarEvento,
  reintentarFoto,
} from '../../data/mejora/repositorio-fotos';
import type { Inmueble } from '../../domain/inmueble';
import { aplicarConfigRemota } from '../../data/mejora/cliente-mejora';

// Estado de fotos mejoradas + config. La red vive en `cliente-mejora`;
// este hook solo orquesta estado local e IndexedDB.
export function useFotosMejora(inmuebles: Inmueble[]) {
  const [fotos, setFotos] = useState<FotoMejora[]>([]);
  const [config, setConfig] = useState<ConfigMejora>(() => leerConfig());
  const [historial, setHistorial] = useState<EventoMejora[]>(() => listarHistorial());
  const [aviso, setAviso] = useState<string | null>(null);
  const ritmoAplicado = useRef(false);

  /* El backend arranca con sus env por defecto: al montar se le empuja la
   * config guardada para que lo que pone la UI mande sin reiniciar nada.
   * Silencioso si el backend está caído (ya avisa la cola al intentarlo). */
  useEffect(() => {
    if (ritmoAplicado.current) return;
    ritmoAplicado.current = true;
    void aplicarConfigRemota(leerConfig()).catch(() => undefined);
  }, []);

  // Firma barata del contenido: las fotos añadidas a un inmueble existente
  // (mismo número de inmuebles) también deben importarse como originales, y
  // un reorden sin cambiar el total debe reimportar (los `orden` cambiaron).
  const firmaFotos = inmuebles
    .map((i) => `${i.id}:${i.fotos.map((f) => f.slice(-40)).join(',')}`)
    .join('|');

  useEffect(() => {
    let vivo = true;
    listarFotos().then((inicial) => {
      if (!vivo) return;
      /* Huérfanas de la era local (su inmueble ya no existe en la API): se
       * purgan solo con la lista ya cargada, nunca en el arranque vacío. */
      const conocidas = new Set(inmuebles.map((i) => i.id));
      const idsHuerfanos = [
        ...new Set(
          inicial.filter((f) => inmuebles.length > 0 && !conocidas.has(f.inmuebleId)).map((f) => f.inmuebleId),
        ),
      ];
      const base = idsHuerfanos.length > 0 ? inicial.filter((f) => conocidas.has(f.inmuebleId)) : inicial;
      if (idsHuerfanos.length > 0) {
        void Promise.all(idsHuerfanos.map((id) => borrarFotosDeInmueble(id).catch(() => undefined)));
      }
      setFotos(base);
      // Migración diferida: purga duplicadas del respaldo y huérfanas con
      // original muerta (404), importa originales nuevos sin bloquear y
      // funde las mejoras que ya guarda el servidor (no se re-encolan).
      purgarMejoradasDuplicadas(inmuebles, base)
        .then((purgadas) => purgarHuerfanas(inmuebles, purgadas))
        .then((vigentes) =>
          importarOriginales(inmuebles, vigentes).then((creadas) =>
            importarMejoradasServidor(inmuebles, [...vigentes, ...creadas]),
          ),
        )
        .then((finales) => {
          if (vivo) setFotos(finales);
        })
        .catch(() => {
          if (vivo) setAviso('No se pudo guardar la copia original (IndexedDB no disponible).');
        });
    });
    return () => {
      vivo = false;
    };
    // Reimporta cuando cambia el contenido de fotos (añadidas, quitadas,
    // sustituidas o reordenadas).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [firmaFotos]);

  const guardar = useCallback(
    (nueva: ConfigMejora) => {
      guardarConfig(nueva);
      setConfig({ ...nueva });
      /* El ritmo vive en el backend: se empuja en caliente para que lo
       * guardado aplique ya, sin reiniciar el servidor. Si no hay backend,
       * aviso visible (se aplicará al arrancar). */
      void aplicarConfigRemota(nueva).then((r) => {
        if (!r.ok) setAviso(r.detalle ?? 'No se pudo aplicar el ritmo al backend.');
      });
    },
    [],
  );

  const marcar = useCallback(async (foto: FotoMejora, parche: ParcheFoto) => {
    try {
      const actualizada = await marcarEstado(foto, parche);
      setFotos((prev) => prev.map((f) => (f.id === foto.id ? actualizada : f)));
      return actualizada;
    } catch {
      setAviso('No se pudo actualizar la foto. Inténtalo de nuevo.');
      return foto;
    }
  }, []);

  const reintentar = useCallback(
    async (foto: FotoMejora) => marcar(foto, { estado: 'pendiente', error: null }),
    [marcar],
  );

  const refrescar = useCallback(async () => {
    setFotos(await listarFotos());
  }, []);

  const borrarDeInmueble = useCallback(async (inmuebleId: string) => {
    await borrarFotosDeInmueble(inmuebleId);
    setFotos((prev) => prev.filter((f) => f.inmuebleId !== inmuebleId));
  }, []);

  const registrar = useCallback((foto: FotoMejora, tipo: TipoEventoMejora, detalle: string | null) => {
    setHistorial(registrarEvento(foto.id, foto.inmuebleId, tipo, detalle));
  }, []);

  const vaciarHistorial = useCallback(() => {
    limpiarHistorial();
    setHistorial([]);
  }, []);

  return { fotos, config, guardar, marcar, reintentar, refrescar, borrarDeInmueble, historial, registrar, vaciarHistorial, aviso, reintentarFoto };
}
