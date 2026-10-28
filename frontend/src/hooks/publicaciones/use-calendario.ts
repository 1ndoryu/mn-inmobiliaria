/* [28AA-6] Estado del calendario de redes: mes visible y sus publicaciones.
 * Cada acción vuelve a pedir el mes tras guardar, para que la vista muestre
 * lo que tiene el servidor y no una copia local que pueda divergir. */
import { useEffect, useState } from 'react';
import { listarPublicaciones } from '../../data/publicaciones/api';
import type { Publicacion } from '../../domain/redes/publicacion-redes';

/* Fecha local como `YYYY-MM-DD` (el formato que espera la API; sin pasar por UTC). */
export function aIso(f: Date): string {
  const mes = String(f.getMonth() + 1).padStart(2, '0');
  const dia = String(f.getDate()).padStart(2, '0');
  return `${f.getFullYear()}-${mes}-${dia}`;
}

/* Primer y último día del mes de `mes`. */
export function rangoDelMes(mes: Date): { desde: string; hasta: string } {
  const primero = new Date(mes.getFullYear(), mes.getMonth(), 1);
  const ultimo = new Date(mes.getFullYear(), mes.getMonth() + 1, 0);
  return { desde: aIso(primero), hasta: aIso(ultimo) };
}

/* Primer día del mes actual, para arrancar la vista sin depender de la hora. */
function mesActual(): Date {
  const hoy = new Date();
  return new Date(hoy.getFullYear(), hoy.getMonth(), 1);
}

export function mensajeError(e: unknown, respaldo: string): string {
  return e instanceof Error ? e.message : respaldo;
}

export function useCalendario() {
  const [mes, setMes] = useState(mesActual);
  const [publicaciones, setPublicaciones] = useState<Publicacion[]>([]);
  const [cargando, setCargando] = useState(true);
  const [error, setError] = useState<string | null>(null);
  /* Contador para volver a pedir el mes tras cada cambio. */
  const [version, setVersion] = useState(0);

  useEffect(() => {
    let vigente = true;
    const { desde, hasta } = rangoDelMes(mes);
    listarPublicaciones(desde, hasta)
      .then((lista) => {
        if (!vigente) return;
        setPublicaciones(lista);
        setError(null);
      })
      .catch((e: unknown) => {
        if (vigente) setError(mensajeError(e, 'No se pudo cargar el calendario.'));
      })
      .finally(() => {
        if (vigente) setCargando(false);
      });
    return () => {
      vigente = false;
    };
  }, [mes, version]);

  function cambiarMes(desplazamiento: number) {
    setMes((m) => new Date(m.getFullYear(), m.getMonth() + desplazamiento, 1));
    setCargando(true);
  }

  function irAHoy() {
    setMes(mesActual());
    setCargando(true);
  }

  /* Vuelve a pedir el mes, p. ej. tras editar una publicación desde el editor. */
  function recargar() {
    setCargando(true);
    setVersion((v) => v + 1);
  }

  /* Ejecuta una acción del servidor: el error se muestra en la vista, no se traga,
   * y si sale bien se vuelve a pedir el mes. Devuelve el resultado o `undefined`. */
  async function operar<T>(accion: () => Promise<T>, respaldo: string): Promise<T | undefined> {
    setError(null);
    try {
      const resultado = await accion();
      setCargando(true);
      setVersion((v) => v + 1);
      return resultado;
    } catch (e) {
      setError(mensajeError(e, respaldo));
      return undefined;
    }
  }

  return { mes, cambiarMes, irAHoy, recargar, publicaciones, cargando, error, setError, operar };
}
