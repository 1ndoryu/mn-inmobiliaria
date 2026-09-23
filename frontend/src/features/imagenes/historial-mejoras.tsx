import { History, Trash2 } from 'lucide-react';
import type { EventoMejora, TipoEventoMejora } from '@/domain/historial-mejora';
import { formatearFecha } from '@/domain/inmueble';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

const ETIQUETA: Record<TipoEventoMejora, string> = {
  encolada: 'Encolada',
  lista: 'Lista',
  error: 'Error',
  cancelado: 'Cancelada',
};

function variante(tipo: TipoEventoMejora): 'default' | 'secondary' | 'destructive' {
  if (tipo === 'lista') return 'default';
  if (tipo === 'error') return 'destructive';
  return 'secondary';
}

/* Historial visible de mejoras: qué foto, de qué inmueble, cuándo y cómo
 * terminó. Solo lectura: no toca fotos ni cola. Máx 200 entradas. */
export function HistorialMejoras(props: {
  eventos: EventoMejora[];
  tituloDe: (inmuebleId: string) => string;
  alLimpiar: () => void;
}) {
  const { eventos, tituloDe, alLimpiar } = props;
  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center gap-2">
        <h2 className="flex items-center gap-1.5 text-base font-semibold">
          <History className="h-4 w-4" /> Historial
        </h2>
        <span className="text-xs text-muted-foreground">{eventos.length} evento{eventos.length === 1 ? '' : 's'}</span>
        {eventos.length > 0 && (
          <Button variant="ghost" size="sm" className="ml-auto h-7 px-2 text-xs" onClick={alLimpiar} title="Vaciar historial">
            <Trash2 className="h-3.5 w-3.5" /> Vaciar
          </Button>
        )}
      </div>
      {eventos.length === 0 ? (
        <p className="rounded-lg border border-dashed px-3 py-6 text-center text-sm text-muted-foreground">
          Aún no hay movimientos. Aquí verás cada foto encolada, completada o fallida con su fecha.
        </p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {eventos.slice(0, 60).map((e) => (
            <li key={e.id} className="flex flex-wrap items-center gap-2 rounded-md border px-3 py-2 text-sm">
              <Badge variant={variante(e.tipo)}>{ETIQUETA[e.tipo]}</Badge>
              <span className="font-medium">{tituloDe(e.inmuebleId)}</span>
              <span className="text-xs text-muted-foreground">{formatearFecha(e.fecha)}</span>
              {e.detalle && <span className="w-full text-xs text-muted-foreground">{e.detalle}</span>}
            </li>
          ))}
        </ul>
      )}
      {eventos.length > 60 && (
        <p className="text-xs text-muted-foreground">Mostrando las 60 más recientes de {eventos.length}.</p>
      )}
    </section>
  );
}
