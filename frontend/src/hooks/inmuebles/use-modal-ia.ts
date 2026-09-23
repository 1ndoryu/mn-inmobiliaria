import { useEffect, useRef, useState } from 'react';
import { ficherosADataUrls } from '@/data/inmuebles/imagenes';
import { organizarConIA, type ResultadoIA } from '@/data/ia/ia';

/* Redacción con IA (3 useState): entrada `{texto, fotos}` |
 * marcha `{fase, resultado}` | `arrastrando`. */

type Fase = 'editar' | 'pensando' | 'previa';

interface Entrada {
  texto: string;
  fotos: string[];
}

interface Marcha {
  fase: Fase;
  resultado: ResultadoIA | null;
}

export function useModalIA(
  abierto: boolean,
  textoInicial: string,
  fotosIniciales: string[],
  alCambiarAbierto: (abierto: boolean) => void,
) {
  const [entrada, setEntrada] = useState<Entrada>({ texto: '', fotos: [] });
  const [marcha, setMarcha] = useState<Marcha>({ fase: 'editar', resultado: null });
  const [arrastrando, setArrastrando] = useState(false);
  const inputFotos = useRef<HTMLInputElement>(null);
  const abortar = useRef<AbortController | null>(null);

  // Cada apertura parte de lo que ya haya en la ficha (no se pierde nada).
  useEffect(() => {
    if (abierto) {
      setEntrada({ texto: textoInicial, fotos: fotosIniciales });
      setMarcha({ fase: 'editar', resultado: null });
      abortar.current?.abort();
      abortar.current = null;
    }
  }, [abierto, textoInicial, fotosIniciales]);

  useEffect(() => () => abortar.current?.abort(), []);

  const cerrar = (v: boolean) => {
    if (!v && marcha.fase === 'pensando') abortar.current?.abort();
    alCambiarAbierto(v);
  };

  const setTexto = (texto: string) => setEntrada((e) => ({ ...e, texto }));

  const quitarFoto = (foto: string) => setEntrada((e) => ({ ...e, fotos: e.fotos.filter((x) => x !== foto) }));

  const anadirFotos = async (files: FileList | File[] | null) => {
    if (!files || files.length === 0) return;
    const { urls } = await ficherosADataUrls(Array.from(files));
    if (urls.length > 0) {
      setEntrada((e) => ({ ...e, fotos: [...e.fotos, ...urls] }));
      setMarcha((m) => ({ ...m, resultado: null }));
    }
    if (inputFotos.current) inputFotos.current.value = '';
  };

  const organizar = async () => {
    abortar.current?.abort();
    const ctrl = new AbortController();
    abortar.current = ctrl;
    setMarcha((m) => ({ ...m, fase: 'pensando', resultado: null }));
    const r = await organizarConIA({ texto: entrada.texto, fotos: entrada.fotos, signal: ctrl.signal });
    abortar.current = null;
    setMarcha({ resultado: r, fase: r.ok ? 'previa' : 'editar' });
  };

  return {
    texto: entrada.texto,
    fotos: entrada.fotos,
    fase: marcha.fase,
    resultado: marcha.resultado,
    arrastrando,
    setArrastrando,
    setTexto,
    setFase: (fase: Fase) => setMarcha((m) => ({ ...m, fase })),
    quitarFoto,
    inputFotos,
    cerrar,
    anadirFotos,
    organizar,
  };
}
