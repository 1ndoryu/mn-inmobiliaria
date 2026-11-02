/* [28AA-6] Estado del «Publicar hoy»: la lista de inmuebles publicables con su última
 * salida, y la creación del borrador del inmueble elegido. La vista solo pinta. */
import { useState } from 'react';
import { crearPublicacion, ultimasPublicaciones } from '../../data/publicaciones/api';
import type { Publicacion } from '../../domain/redes/publicacion-redes';
import { aIso, mensajeError, type useCalendario } from './use-calendario';

type Calendario = Pick<ReturnType<typeof useCalendario>, 'operar' | 'setError'>;

export function useEleccionHoy(cal: Calendario, alCrear: (publicacion: Publicacion) => void) {
  const [eligiendo, setEligiendo] = useState(false);
  /* `null` mientras se piden; luego, la última salida de cada inmueble publicado. */
  const [ultimas, setUltimas] = useState<Record<string, string> | null>(null);
  /* Evita crear dos borradores con un doble clic mientras llega la respuesta. */
  const [creando, setCreando] = useState(false);

  async function abrir() {
    setEligiendo(true);
    setUltimas(null);
    try {
      setUltimas(await ultimasPublicaciones());
    } catch (e) {
      setEligiendo(false);
      cal.setError(mensajeError(e, 'No se pudieron cargar las publicaciones.'));
    }
  }

  async function elegir(inmuebleId: string) {
    /* El borrador nace con la fecha de hoy: así sale en la rejilla del día. */
    setCreando(true);
    const creada = await cal.operar(
      () => crearPublicacion(inmuebleId, aIso(new Date())),
      'No se pudo crear la publicación.',
    );
    setCreando(false);
    if (!creada) return;
    setEligiendo(false);
    alCrear(creada);
  }

  return { eligiendo, ultimas, creando, abrir, cerrar: () => setEligiendo(false), elegir };
}
