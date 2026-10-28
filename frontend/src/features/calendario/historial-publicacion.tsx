/* [28AA-6] Historial de salidas de un inmueble: una línea por cada vez que salió publicado. */
import { useEffect, useState } from 'react';
import { historialDe } from '@/data/publicaciones/api';
import type { Publicacion } from '@/domain/redes/publicacion-redes';
import { mensajeError } from '@/hooks/publicaciones/use-calendario';

interface HistorialPublicacionProps {
  inmuebleId: string;
}

export function HistorialPublicacion({ inmuebleId }: HistorialPublicacionProps) {
  const [historial, setHistorial] = useState<Publicacion[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    /* `vigente` evita escribir el estado de un inmueble que ya no se muestra. */
    let vigente = true;
    historialDe(inmuebleId)
      .then((lista) => {
        if (vigente) setHistorial(lista);
      })
      .catch((e: unknown) => {
        if (vigente) setError(mensajeError(e, 'No se pudo cargar el historial.'));
      });
    return () => {
      vigente = false;
    };
  }, [inmuebleId]);

  if (error) return <p className="text-sm text-destructive">{error}</p>;
  if (!historial) return <p className="text-sm text-muted-foreground">Cargando historial…</p>;
  if (historial.length === 0) {
    return <p className="text-sm text-muted-foreground">Todavía no ha salido publicado.</p>;
  }

  return (
    <ul className="flex flex-col divide-y text-sm">
      {historial.map((p) => (
        <li key={p.id} className="py-1.5">
          {p.publicadaEn ? new Date(p.publicadaEn).toLocaleString('es', { dateStyle: 'medium', timeStyle: 'short' }) : '—'}
        </li>
      ))}
    </ul>
  );
}
