import { Search } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select } from '@/components/ui/select';
import type { EstadoInmueble } from '../domain/inmueble';

/* Barra de búsqueda y filtro de estado de la lista de administración.
 * Se separa de `app-admin.tsx` para mantener ese componente bajo el límite de líneas. */
export function FiltrosInmuebles({
  busqueda,
  alCambiarBusqueda,
  filtroEstado,
  alCambiarFiltro,
}: {
  busqueda: string;
  alCambiarBusqueda: (valor: string) => void;
  filtroEstado: 'todos' | EstadoInmueble;
  alCambiarFiltro: (estado: 'todos' | EstadoInmueble) => void;
}) {
  return (
    <div className="mb-4 flex flex-col gap-2 sm:flex-row">
      <div className="relative flex-1">
        <Search className="absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          value={busqueda}
          onChange={(e) => alCambiarBusqueda(e.target.value)}
          placeholder="Buscar por título, ubicación o descripción…"
          className="pl-9"
        />
      </div>
      <Select
        sinBase
        value={filtroEstado}
        onChange={(e) => alCambiarFiltro(e.target.value as 'todos' | EstadoInmueble)}
        className="flex h-9 rounded-md border border-input bg-background px-3 py-1 text-sm capitalize focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30"
      >
        <option value="todos">Todos los estados</option>
        <option value="disponible">disponible</option>
        <option value="reservado">reservado</option>
        <option value="vendido">vendido</option>
        <option value="alquilado">alquilado</option>
      </Select>
    </div>
  );
}

/* Pie de la lista paginada: rango mostrado y botones Anterior/Siguiente. */
export function PaginacionInmuebles({
  pagina,
  totalPaginas,
  tamano,
  total,
  alCambiar,
}: {
  pagina: number;
  totalPaginas: number;
  tamano: number;
  total: number;
  alCambiar: (pagina: number) => void;
}) {
  return (
    <div className="mt-4 flex items-center justify-between gap-3 text-sm">
      <span className="text-muted-foreground">
        {(pagina - 1) * tamano + 1}–{Math.min(pagina * tamano, total)} de {total}
      </span>
      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" disabled={pagina <= 1} onClick={() => alCambiar(pagina - 1)}>
          Anterior
        </Button>
        <span className="tabular-nums">
          {pagina} / {totalPaginas}
        </span>
        <Button variant="outline" size="sm" disabled={pagina >= totalPaginas} onClick={() => alCambiar(pagina + 1)}>
          Siguiente
        </Button>
      </div>
    </div>
  );
}
