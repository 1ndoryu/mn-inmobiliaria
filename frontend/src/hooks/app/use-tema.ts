import { useCallback, useEffect, useState } from 'react';
import { aplicarTema, guardarTema, leerTemaGuardado, type Tema } from '../../app/tema';

const ORDEN: Tema[] = ['sistema', 'claro', 'oscuro'];

// Tema global: persiste la elección y reacciona al cambio del sistema
// cuando el modo es "sistema".
export function useTema() {
  const [tema, setTemaState] = useState<Tema>(() => leerTemaGuardado());

  useEffect(() => {
    aplicarTema(tema);
  }, [tema]);

  useEffect(() => {
    if (tema !== 'sistema') return;
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = () => aplicarTema('sistema');
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, [tema]);

  const setTema = useCallback((t: Tema) => {
    guardarTema(t);
    setTemaState(t);
  }, []);

  const ciclar = useCallback(() => {
    setTemaState((actual) => {
      const siguiente = ORDEN[(ORDEN.indexOf(actual) + 1) % ORDEN.length];
      guardarTema(siguiente);
      return siguiente;
    });
  }, []);

  return { tema, setTema, ciclar };
}
