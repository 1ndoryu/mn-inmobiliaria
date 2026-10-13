/* [13AA-12] Diferencias de un inmueble con el servidor, con el valor de cada lado.
 * Solo lectura: no sube ni baja nada. Se recarga si cambia la revisión mientras
 * está abierto. El padre lo monta con `key` por inmueble (estado limpio al abrir). */
import { useEffect, useState } from 'react';
import { diferenciasSync, type DiferenciasSync } from '@/data/inmuebles/sync';
import { etiquetaEstado, etiquetaLado, nombreClave } from '@/domain/sync/sync-inmueble';
import { Badge } from '@/components/ui/badge';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';

interface Props {
  /* `null` = cerrado. */
  slug: string | null;
  titulo: string;
  revision: number;
  alCerrar: () => void;
}

/* Las huellas de foto no son texto legible: se acortan; «presente» y «sin-archivo» se traducen. */
function describirValor(clave: string, valor: string): string {
  if (!clave.startsWith('foto:')) return valor || '(vacío)';
  if (valor === 'presente') return 'Presente (sin comparar bytes)';
  if (valor === 'sin-archivo') return 'Archivo no encontrado';
  return `Huella ${valor.slice(0, 12)}…`;
}

function ValorSync({ clave, valor }: { clave: string; valor: string | null }) {
  if (valor === null) return <span className="italic text-muted-foreground">No existe</span>;
  return (
    <p title={valor} className="max-h-32 max-w-xs overflow-auto whitespace-pre-wrap break-words text-sm">
      {describirValor(clave, valor)}
    </p>
  );
}

export function ModalDiferenciasSync({ slug, titulo, revision, alCerrar }: Props) {
  const [datos, setDatos] = useState<DiferenciasSync | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (slug === null) return;
    let vigente = true;
    diferenciasSync(slug)
      .then((d) => {
        if (!vigente) return;
        setDatos(d);
        setError(null);
      })
      .catch((e: unknown) => {
        if (!vigente) return;
        setError(e instanceof Error ? e.message : 'No se pudieron leer las diferencias.');
      });
    return () => {
      vigente = false;
    };
  }, [slug, revision]);

  const etiqueta = datos ? etiquetaEstado(datos.resultado.estado, datos.resultado.estimado) : null;

  return (
    <Dialog open={slug !== null} onOpenChange={(abierto) => !abierto && alCerrar()}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>Diferencias con el servidor</DialogTitle>
          <DialogDescription>{titulo || 'Sin título'}</DialogDescription>
        </DialogHeader>
        {error && (
          <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</p>
        )}
        {!datos && !error && <p className="text-sm text-muted-foreground">Cargando diferencias…</p>}
        {datos && etiqueta && (
          <div className="space-y-3">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="secondary" className={etiqueta.clase}>
                {etiqueta.texto}
              </Badge>
              {datos.resultado.estimado && (
                <span className="text-xs text-muted-foreground">
                  Sin base previa: la dirección se estima por la última edición de cada lado.
                </span>
              )}
            </div>
            {datos.filas.length === 0 ? (
              <p className="text-sm text-muted-foreground">Sin diferencias de contenido.</p>
            ) : (
              <div className="rounded-lg border">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Campo</TableHead>
                      <TableHead>Cambió</TableHead>
                      <TableHead>Local</TableHead>
                      <TableHead>Servidor</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {datos.filas.map((fila) => (
                      <TableRow key={fila.clave}>
                        <TableCell className="whitespace-nowrap font-medium">{nombreClave(fila.clave)}</TableCell>
                        <TableCell>{etiquetaLado(fila.lado)}</TableCell>
                        <TableCell>
                          <ValorSync clave={fila.clave} valor={fila.local} />
                        </TableCell>
                        <TableCell>
                          <ValorSync clave={fila.clave} valor={fila.servidor} />
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
