/* [28AA-6] Estado y acciones del editor de una publicación: texto, destinos,
 * generación con IA, guardado y borrado. El componente solo pinta. */
import { useEffect, useRef, useState } from 'react';
import { generarDescripcionRedes, normalizarTags } from '@/data/ia/descripcion-redes';
import { actualizarPublicacion, borrarPublicacion } from '@/data/publicaciones/api';
import type { Inmueble } from '@/domain/inmueble';
import type { Destinos, Publicacion } from '@/domain/redes/publicacion-redes';
import { mensajeError } from '@/hooks/publicaciones/use-calendario';

interface Opciones {
  publicacion: Publicacion;
  inmueble: Inmueble | undefined;
  alCambiar: (publicacion: Publicacion) => void;
  alBorrar: () => void;
}

export function useEditorPublicacion({ publicacion, inmueble, alCambiar, alBorrar }: Opciones) {
  const [pub, setPub] = useState(publicacion);
  const [destinos, setDestinos] = useState<Destinos>(publicacion.destinos);
  const [descripcion, setDescripcion] = useState(publicacion.descripcion);
  const [tagsTexto, setTagsTexto] = useState(publicacion.tags.join(', '));
  /* Prompt y modelo de la última generación con IA: viajan con el siguiente guardado. */
  const [generacion, setGeneracion] = useState<{ prompt: string; modelo: string } | null>(null);
  const [generando, setGenerando] = useState(false);
  const [guardando, setGuardando] = useState(false);
  const [confirmarBorrado, setConfirmarBorrado] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const controlRef = useRef<AbortController | null>(null);

  /* Cancela una generación en curso si el editor se cierra. */
  useEffect(() => () => controlRef.current?.abort(), []);

  const ocupado = generando || guardando;
  const publicada = pub.estado === 'publicada';

  async function generar() {
    if (!inmueble) return;
    const control = new AbortController();
    controlRef.current = control;
    setGenerando(true);
    setError(null);
    try {
      const r = await generarDescripcionRedes(inmueble, control.signal);
      if (!r.ok) {
        setError(r.motivo);
        return;
      }
      setDescripcion(r.datos.descripcion);
      setTagsTexto(r.datos.tags.join(', '));
      setGeneracion({ prompt: r.prompt, modelo: r.modelo });
    } catch (e) {
      setError(mensajeError(e, 'No se pudo generar la descripción.'));
    } finally {
      setGenerando(false);
    }
  }

  async function guardar() {
    setGuardando(true);
    setError(null);
    try {
      const actualizada = await actualizarPublicacion(pub.id, {
        descripcion,
        tags: normalizarTags(tagsTexto.split(/[,\n]/)),
        programadaPara: pub.programadaPara,
        destinos,
        promptUsado: generacion?.prompt,
        modelo: generacion?.modelo,
      });
      setPub(actualizada);
      setDescripcion(actualizada.descripcion);
      setTagsTexto(actualizada.tags.join(', '));
      setGeneracion(null);
      alCambiar(actualizada);
    } catch (e) {
      setError(mensajeError(e, 'No se pudieron guardar los cambios.'));
    } finally {
      setGuardando(false);
    }
  }

  /* Borrar pide confirmación: el primer clic solo cambia el texto del botón. */
  async function borrar() {
    if (!confirmarBorrado) {
      setConfirmarBorrado(true);
      return;
    }
    setGuardando(true);
    setError(null);
    try {
      await borrarPublicacion(pub.id);
      alBorrar();
    } catch (e) {
      setError(mensajeError(e, 'No se pudo borrar la publicación.'));
      setGuardando(false);
    }
  }

  return {
    pub,
    setPub,
    destinos,
    setDestinos,
    descripcion,
    setDescripcion,
    tagsTexto,
    setTagsTexto,
    generando,
    guardando,
    ocupado,
    publicada,
    confirmarBorrado,
    error,
    generar,
    guardar,
    borrar,
  };
}
