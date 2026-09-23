import { useEffect, useRef, useState } from 'react';
import {
  DRAFT_VACIO,
  draftAInmueble,
  inmuebleADraft,
  validarDraft,
  type ErroresDraft,
  type Inmueble,
  type InmuebleDraft,
} from '@/domain/inmueble';
import { ficherosADataUrls } from '@/data/inmuebles/imagenes';
import type { FichaIA } from '@/data/ia/ia';
import { aplicarFichaAlDraft } from '@/hooks/inmuebles/aplicar-ficha-ia';

/* Formulario nuevo/edición (3 useState): `formEdicion` |
 * avisos `{errores, avisoFotos}` | `iaAbierto`.
 * El borrador persistente (modo "nuevo") vive en el padre para sobrevivir
 * al modal; aquí solo vive el formulario de edición. */

interface Avisos {
  errores: ErroresDraft;
  avisoFotos: string | null;
}

interface Opciones {
  abierto: boolean;
  editando: Inmueble | null;
  borrador: InmuebleDraft;
  setBorrador: (d: InmuebleDraft) => void;
  alCambiarAbierto: (abierto: boolean) => void;
  limpiarTrasGuardar: () => void;
  onGuardarNuevo: (inmueble: Inmueble) => Promise<boolean>;
  onGuardarEdicion: (inmueble: Inmueble) => Promise<boolean>;
}

export function useModalInmueble(opciones: Opciones) {
  const { abierto, editando, borrador, setBorrador } = opciones;
  const esEdicion = editando !== null;
  const [formEdicion, setFormEdicion] = useState<InmuebleDraft>(DRAFT_VACIO);
  const [avisos, setAvisos] = useState<Avisos>({ errores: {}, avisoFotos: null });
  const [iaAbierto, setIaAbierto] = useState(false);
  const inputFotos = useRef<HTMLInputElement>(null);

  // Formulario activo: borrador persistente (nuevo) o estado local (edición).
  const form = esEdicion ? formEdicion : borrador;
  const setForm = esEdicion ? setFormEdicion : setBorrador;

  useEffect(() => {
    if (abierto && editando) {
      setFormEdicion(inmuebleADraft(editando));
      setAvisos({ errores: {}, avisoFotos: null });
    }
    if (abierto && !editando) {
      setAvisos({ errores: {}, avisoFotos: null });
    }
  }, [abierto, editando]);

  const cambiar = <K extends keyof InmuebleDraft>(campo: K, valor: InmuebleDraft[K]) => {
    setForm({ ...form, [campo]: valor });
    setAvisos((a) => ({ ...a, errores: { ...a.errores, [campo]: undefined } }));
  };

  const anadirFotos = async (files: FileList | null) => {
    if (!files || files.length === 0) return;
    const { urls, errores } = await ficherosADataUrls(Array.from(files));
    if (urls.length > 0) setForm({ ...form, fotos: [...form.fotos, ...urls] });
    setAvisos((a) => ({ ...a, avisoFotos: errores.length > 0 ? errores.join(' ') : null }));
    if (inputFotos.current) inputFotos.current.value = '';
  };

  const quitarFoto = (idx: number) => setForm({ ...form, fotos: form.fotos.filter((_, j) => j !== idx) });

  /* La principal es la primera: la web pública muestra fotos[0] y el
   * backend ordena por `orden`, así que moverla al frente basta (al
   * guardar, `sincronizarFotos` la re-sube con orden 0). */
  const hacerPrincipal = (idx: number) => {
    if (idx <= 0 || idx >= form.fotos.length) return;
    setForm({ ...form, fotos: [form.fotos[idx], ...form.fotos.filter((_, j) => j !== idx)] });
  };

  const guardar = async () => {
    const e = validarDraft(form);
    setAvisos((a) => ({ ...a, errores: e }));
    if (Object.keys(e).length > 0) return;
    if (esEdicion && editando) {
      // Si falla (p. ej. almacén no disponible), el banner de error lo explica
      // y el modal sigue abierto con el formulario intacto.
      const ok = await opciones.onGuardarEdicion(draftAInmueble(form, editando));
      if (ok) opciones.alCambiarAbierto(false);
    } else {
      const ok = await opciones.onGuardarNuevo(draftAInmueble(form));
      if (ok) {
        await opciones.limpiarTrasGuardar();
        opciones.alCambiarAbierto(false);
      }
    }
  };

  /* La IA solo rellena los campos que pudo determinar (fusión pura en
   * `aplicar-ficha-ia`); las fotos nuevas se añaden sin duplicar. */
  const aplicarFichaIA = (ficha: FichaIA, fotosIA: string[]) => {
    setForm(aplicarFichaAlDraft(form, ficha, fotosIA));
    setAvisos((a) => ({ ...a, errores: {} }));
  };

  return {
    esEdicion,
    form,
    errores: avisos.errores,
    avisoFotos: avisos.avisoFotos,
    iaAbierto,
    setIaAbierto,
    inputFotos,
    cambiar,
    anadirFotos,
    quitarFoto,
    hacerPrincipal,
    guardar,
    aplicarFichaIA,
  };
}
