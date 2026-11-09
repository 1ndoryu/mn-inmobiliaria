/* [30AA-1] Historial de textos: cada cambio de título o descripción de una
 * variante, con lo anterior, lo nuevo y si lo hizo la IA o a mano. Es la
 * prueba de que la v2 no inventó nada: se puede comparar línea a línea. */
import { Badge } from '@/components/ui/badge';
import type { EntradaHistorial } from '@/domain/versiones';

function fecha(iso: string): string {
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

export function PestanaHistorial({ entradas }: { entradas: EntradaHistorial[] }) {
  if (entradas.length === 0) {
    return (
      <p className="rounded-md border bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        Todavía no hay cambios: el historial se llena al generar o editar una versión.
      </p>
    );
  }

  return (
    <ul className="grid gap-2">
      {entradas.map((e) => (
        <li key={e.id} className="rounded-md border p-3 text-sm">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="outline">v{e.version}</Badge>
            <span className="font-medium">{e.campo === 'titulo' ? 'Título' : 'Descripción'}</span>
            <Badge variant={e.origen === 'ia' ? 'secondary' : 'outline'}>{e.origen === 'ia' ? 'IA' : 'A mano'}</Badge>
            {e.modelo && <span className="text-xs text-muted-foreground">{e.modelo}</span>}
            <span className="ml-auto text-xs text-muted-foreground">{fecha(e.creado_en)}</span>
          </div>
          <p className="mt-2 line-through opacity-60">{e.valor_anterior || '—'}</p>
          <p className="mt-1">{e.valor_nuevo || '—'}</p>
        </li>
      ))}
    </ul>
  );
}
