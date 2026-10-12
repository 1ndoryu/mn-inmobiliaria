/* [169A-2] Panel admin de solicitudes "Publicar mi inmueble": lista las
 * propuestas de clientes por estado y mueve su revisión. La lógica de
 * estado vive en `useSolicitudes`; aquí solo se pinta. */
import { Button } from '@/components/ui/button';
import { Select } from '@/components/ui/select';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { urlFotoSolicitud } from '../../data/publicar/api';
import { ESTADOS_SOLICITUD, type EstadoSolicitud, type Solicitud } from '../../domain/solicitud';
import { type FiltroEstado, POR_PAGINA, useSolicitudes } from '../../hooks/solicitudes/use-solicitudes';

const FILTROS: { valor: FiltroEstado; texto: string }[] = [
  { valor: 'todos', texto: 'Todas' },
  ...ESTADOS_SOLICITUD.map((e) => ({ valor: e, texto: e })),
];

function fechaCorta(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString('es-VE', { dateStyle: 'short', timeStyle: 'short' });
}

function detalleInmueble(s: Solicitud): string {
  const partes = [s.operacion];
  if (s.precioEstimado !== null) partes.push(s.precioEstimado.toLocaleString('es-VE'));
  if (s.puestos > 0) partes.push(`${s.puestos} puesto${s.puestos === 1 ? '' : 's'}`);
  if (s.residencia) partes.push(s.residencia);
  return partes.join(' · ');
}

export function VistaSolicitudes() {
  const { filtro, cambiarFiltro, pagina, irAPagina, datos, cargando, error, guardandoId, cambiarEstado, total, numPaginas } =
    useSolicitudes();

  return (
    <section>
      <div className="mb-2 flex flex-wrap items-center gap-2">
        {FILTROS.map((f) => (
          <Button
            key={f.valor}
            variant={filtro === f.valor ? 'default' : 'outline'}
            size="sm"
            className="capitalize"
            onClick={() => cambiarFiltro(f.valor)}
          >
            {f.texto}
          </Button>
        ))}
        <span className="text-xs text-muted-foreground">
          {cargando ? 'Cargando…' : `${total} solicitud${total === 1 ? '' : 'es'}`}
        </span>
      </div>
      {error && (
        <p className="mb-2 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Fecha</TableHead>
            <TableHead>Contacto</TableHead>
            <TableHead>Inmueble</TableHead>
            <TableHead>Descripción</TableHead>
            <TableHead>Fotos</TableHead>
            <TableHead>Estado</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {(datos?.items ?? []).map((s) => (
            <TableRow key={s.id}>
              <TableCell className="whitespace-nowrap">{fechaCorta(s.creadaEn)}</TableCell>
              <TableCell>
                <div className="font-medium">{s.nombre}</div>
                <div className="text-xs text-muted-foreground">{s.telefono}</div>
                {s.email && <div className="text-xs text-muted-foreground">{s.email}</div>}
              </TableCell>
              <TableCell>
                <div>{s.ubicacion || '—'}</div>
                <div className="text-xs text-muted-foreground">{detalleInmueble(s)}</div>
              </TableCell>
              <TableCell className="max-w-64 truncate" title={s.descripcion}>
                {s.descripcion || '—'}
              </TableCell>
              <TableCell>
                {s.fotos.length === 0 ? (
                  <span className="text-xs text-muted-foreground">Sin fotos</span>
                ) : (
                  <div className="flex flex-wrap gap-2">
                    {s.fotos.map((url, i) => (
                      <a
                        key={url}
                        href={urlFotoSolicitud(url)}
                        target="_blank"
                        rel="noreferrer"
                        className="text-xs underline underline-offset-2"
                      >
                        Foto {i + 1}
                      </a>
                    ))}
                  </div>
                )}
              </TableCell>
              <TableCell>
                <Select
                  sinBase
                  aria-label="Estado de revisión"
                  value={s.estado}
                  disabled={guardandoId === s.id}
                  onChange={(e) => void cambiarEstado(s, e.target.value as EstadoSolicitud)}
                  className="flex h-8 rounded-md border border-input bg-background px-2 text-xs capitalize focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30 disabled:opacity-50"
                >
                  {ESTADOS_SOLICITUD.map((e) => (
                    <option key={e} value={e}>
                      {e}
                    </option>
                  ))}
                </Select>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {datos && !cargando && datos.items.length === 0 && !error && (
        <p className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
          Sin solicitudes en este estado.
        </p>
      )}
      {numPaginas > 1 && (
        <div className="mt-3 flex items-center justify-end gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={pagina <= 1 || cargando}
            onClick={() => irAPagina(pagina - 1)}
          >
            Anterior
          </Button>
          <span className="text-xs text-muted-foreground">
            Página {pagina} de {numPaginas} · {POR_PAGINA} por página
          </span>
          <Button
            variant="outline"
            size="sm"
            disabled={pagina >= numPaginas || cargando}
            onClick={() => irAPagina(pagina + 1)}
          >
            Siguiente
          </Button>
        </div>
      )}
    </section>
  );
}
