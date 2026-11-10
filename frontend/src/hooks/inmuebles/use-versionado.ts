import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  CuentaFacebook,
  EntradaHistorial,
  EstadoVersion,
  FotoVariante,
  VersionesInmueble,
  VinculoFacebook,
} from '../../domain/versiones';
import {
  actualizarCuenta,
  borrarCuenta,
  borrarFotoVariante,
  borrarVersion,
  crearCuenta,
  desvincularCuenta,
  editarVersion,
  generarVersion,
  guardarConfigVersiones,
  leerConfigVersiones,
  leerVersiones,
  listarCuentas,
  subirFotoVariante,
  vincularCuenta,
  vinculosDe,
} from '../../data/inmuebles/versiones-api';
import { enviarFoto, leerTrabajo } from '../../data/mejora/cliente-mejora';
import { dataUrlABytes } from '../../data/inmuebles/api';
import { nuevoId } from '../../domain/foto-mejora';

/* [30AA-1] Estado del versionado de UN inmueble: cuentas de Facebook,
 * variantes de texto (v2+) y fotos variante. Vive aparte del modal para
 * que este solo pinte. */

interface Opciones {
  inmuebleId: string | null;
  /** Fotos del inmueble en orden: URL de la mejorada si existe, si no el original. */
  fotosParaVariante: string[];
  promptVariante: string;
}

export interface ProgresoVariantes {
  total: number;
  hechas: number;
  actual: number | null;
  detalle: string | null;
}

export function useVersionado({ inmuebleId, fotosParaVariante, promptVariante }: Opciones) {
  const [datos, setDatos] = useState<VersionesInmueble | null>(null);
  const [cuentas, setCuentas] = useState<CuentaFacebook[]>([]);
  const [vinculos, setVinculos] = useState<VinculoFacebook[]>([]);
  const [ocupado, setOcupado] = useState(false);
  const [aviso, setAviso] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progreso, setProgreso] = useState<ProgresoVariantes | null>(null);
  /* Una tanda de fotos variante a la vez: el backend ya va de una en una y
   * lanzar dos tandas mezclaría posiciones. */
  const variantesEnCurso = useRef(false);

  const recargar = useCallback(async () => {
    if (!inmuebleId) return;
    try {
      const [v, vincs] = await Promise.all([leerVersiones(inmuebleId), vinculosDe(inmuebleId)]);
      setDatos(v);
      setVinculos(vincs);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'No se pudieron leer las versiones.');
    }
  }, [inmuebleId]);

  useEffect(() => {
    setDatos(null);
    setVinculos([]);
    setAviso(null);
    setError(null);
    void recargar();
    listarCuentas()
      .then(setCuentas)
      .catch(() => setCuentas([]));
  }, [recargar]);

  /* Toda mutación pasa por aquí: bloquea, limpia avisos y recarga al final
   * (la fuente de verdad es el servidor, nunca una copia local). */
  const mutar = useCallback(
    async <T,>(tarea: () => Promise<T>): Promise<T | null> => {
      setOcupado(true);
      setAviso(null);
      setError(null);
      try {
        const r = await tarea();
        await recargar();
        return r;
      } catch (e) {
        setError(e instanceof Error ? e.message : 'La operación falló.');
        return null;
      } finally {
        setOcupado(false);
      }
    },
    [recargar],
  );

  /* ---- Texto ---- */

  const generar = useCallback(
    async (version: number, instruccion = '') => {
      const r = await mutar(() => generarVersion(inmuebleId!, version, instruccion));
      if (r && !r.ok) {
        /* El backend rechaza una variante que inventa datos: se muestra el
         * motivo y no se guarda nada. */
        setAviso(r.motivos.join(' ') || 'La IA no devolvió una variante válida.');
      } else if (r) {
        setAviso(`Versión ${version} generada en borrador. Revísala antes de marcarla lista.`);
      }
    },
    [inmuebleId, mutar],
  );

  const guardarTexto = useCallback(
    async (version: number, titulo: string, descripcion: string, estado?: EstadoVersion) =>
      mutar(() => editarVersion(inmuebleId!, version, { titulo, descripcion, estado })),
    [inmuebleId, mutar],
  );

  const borrarTexto = useCallback(
    async (version: number) => mutar(() => borrarVersion(inmuebleId!, version)),
    [inmuebleId, mutar],
  );

  /* ---- Fotos variante ---- */

  /* Genera la variante de TODAS las fotos del inmueble (o solo las que
   * falten) para la versión pedida. Secuencial a propósito: el backend de
   * mejora ya espacia las peticiones y así se ve el avance foto a foto.
   * [30AA-4] La versión ya no es fija: la pestaña decide cuál se genera. */
  const generarFotos = useCallback(async (version: number) => {
    if (!inmuebleId || fotosParaVariante.length === 0 || variantesEnCurso.current) return;
    variantesEnCurso.current = true;
    const yaHechas = new Set((datos?.fotos ?? []).filter((f) => f.version === version).map((f) => f.orden));
    const pendientes = fotosParaVariante.map((_, i) => i).filter((i) => !yaHechas.has(i));
    setProgreso({ total: pendientes.length, hechas: 0, actual: null, detalle: null });
    setAviso(null);
    setError(null);
    let hechas = 0;
    try {
      for (const orden of pendientes) {
        setProgreso({ total: pendientes.length, hechas, actual: orden, detalle: null });
        const url = fotosParaVariante[orden]!;
        const envio = await enviarFoto(
          `variante-${inmuebleId}-v${version}-${orden}-${nuevoId()}`,
          url,
          promptVariante,
        );
        if (!envio.ok || !envio.jobId) {
          setProgreso({
            total: pendientes.length,
            hechas,
            actual: null,
            detalle: `Foto ${orden + 1}: ${envio.motivo ?? 'no se pudo encolar.'}`,
          });
          return;
        }
        const imagen = await esperarVariante(envio.jobId);
        if (!imagen.ok) {
          setProgreso({
            total: pendientes.length,
            hechas,
            actual: null,
            detalle: `Foto ${orden + 1}: ${imagen.motivo}`,
          });
          return;
        }
        /* DataURL → bytes → subida. La variante se guarda en el servidor
         * para que el escritorio la exporte igual que la v1. */
        const { bytes, extension } = dataUrlABytes(imagen.imagen);
        await subirFotoVariante(inmuebleId, orden, version, bytes, extension, 'gemini', promptVariante);
        hechas += 1;
        setProgreso({ total: pendientes.length, hechas, actual: null, detalle: null });
      }
      await recargar();
      setAviso(`Variantes de la versión ${version} listas: ${hechas} de ${pendientes.length}.`);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Falló la generación de fotos variante.');
    } finally {
      variantesEnCurso.current = false;
      setProgreso(null);
    }
  }, [datos, fotosParaVariante, inmuebleId, promptVariante, recargar]);

  const borrarFoto = useCallback(
    async (orden: number, version: number) =>
      mutar(() => borrarFotoVariante(inmuebleId!, orden, version)),
    [inmuebleId, mutar],
  );

  /* ---- Cuentas de Facebook ---- */

  const regenerarCuentas = useCallback(async () => {
    try {
      setCuentas(await listarCuentas());
    } catch {
      /* Sin cuentas la sección avisa, no rompe el modal. */
    }
  }, []);

  const crear = useCallback(
    async (nombre: string, url: string) => {
      const r = await mutar(() => crearCuenta({ nombre, url }));
      await regenerarCuentas();
      return r;
    },
    [mutar, regenerarCuentas],
  );

  const editarCuenta = useCallback(
    async (id: string, datos2: { nombre: string; url: string; principal: boolean }) => {
      const r = await mutar(() => actualizarCuenta(id, datos2));
      await regenerarCuentas();
      return r;
    },
    [mutar, regenerarCuentas],
  );

  const eliminarCuenta = useCallback(
    async (id: string) => {
      await mutar(() => borrarCuenta(id));
      await regenerarCuentas();
    },
    [mutar, regenerarCuentas],
  );

  const vincular = useCallback(
    async (cuentaId: string, marketplaceId: string | null) => {
      const r = await mutar(() => vincularCuenta(inmuebleId!, cuentaId, marketplaceId));
      if (r) setVinculos(r);
    },
    [inmuebleId, mutar],
  );

  const desvincular = useCallback(
    async (cuentaId: string) => {
      const r = await mutar(() => desvincularCuenta(inmuebleId!, cuentaId));
      if (r) setVinculos(r);
    },
    [inmuebleId, mutar],
  );

  /* ---- Configuración global ---- */

  const fijarConfiguradas = useCallback(
    async (n: number) => {
      await guardarConfigVersiones(n);
      await recargar();
    },
    [recargar],
  );

  const leerConfiguradas = useCallback(async () => {
    try {
      return (await leerConfigVersiones()).versiones;
    } catch {
      return 2;
    }
  }, []);

  return {
    datos,
    cuentas,
    vinculos,
    ocupado,
    aviso,
    error,
    progreso,
    recargar,
    generar,
    guardarTexto,
    borrarTexto,
    generarFotos,
    borrarFoto,
    crear,
    editarCuenta,
    eliminarCuenta,
    vincular,
    desvincular,
    fijarConfiguradas,
    leerConfiguradas,
  };
}

/* Sondea el trabajo de mejora hasta que devuelva la imagen o un error final.
 * Los estados no terminales (encolado con reintento, procesando) se esperan:
 * el backend ya aplica su propio backoff. */
async function esperarVariante(
  jobId: string,
): Promise<{ ok: true; imagen: string } | { ok: false; motivo: string }> {
  const inicio = Date.now();
  let nulos = 0;
  while (Date.now() - inicio < 30 * 60 * 1000) {
    await new Promise((r) => setTimeout(r, 5000));
    const estado = await leerTrabajo(jobId);
    if (!estado) {
      nulos += 1;
      if (nulos >= 24) return { ok: false, motivo: 'sin contacto con el backend durante 2 min.' };
      continue;
    }
    nulos = 0;
    if (estado.estado === 'lista' && estado.imagen) return { ok: true, imagen: estado.imagen };
    if (estado.estado === 'error') return { ok: false, motivo: estado.error ?? 'el backend no pudo generarla.' };
    if (estado.estado === 'cancelado') return { ok: false, motivo: 'cancelada.' };
    if (estado.estado === 'desconocido') return { ok: false, motivo: 'el backend perdió el trabajo.' };
  }
  return { ok: false, motivo: 'se agotó el tiempo de espera (30 min).' };
}

/* Historial → filas legibles para la pestaña. Pura. */
export function filasHistorial(historial: EntradaHistorial[]): EntradaHistorial[] {
  return [...historial].sort((a, b) => b.creado_en.localeCompare(a.creado_en));
}

/* Pares (original, variante) por `orden` para pintar lado a lado. Pura. */
/* [30AA-4] Empareja cada original con su variante DE LA VERSIÓN pedida: la
 * misma foto puede tener v2 y v3 a la vez y cada pestaña enseña la suya. */
export function paresDeFotos(
  fotos: string[],
  variantes: FotoVariante[],
  version = 2,
): { orden: number; original: string; variante: FotoVariante | null }[] {
  const porOrden = new Map(variantes.filter((v) => v.version === version).map((v) => [v.orden, v]));
  return fotos.map((original, orden) => ({ orden, original, variante: porOrden.get(orden) ?? null }));
}
