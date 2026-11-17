/* [30AA-14] Modal de revisión: todas las versiones de las fotos de un inmueble
 * en una rejilla y el % de diferencia de cada una contra su original. La
 * medición la hace el backend (`GET /inmuebles/:id/fotos/versiones`): el front
 * solo pinta. Se abre desde la acción «Revisar versiones» de la fila.
 * Vive en `versionado/` porque la carpeta `inmuebles/` está en su tope de 10
 * archivos (hallazgo del gate, 2026-11-16). */
import { useEffect, useState } from 'react';
import { Layers } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { leerRevisionFotos } from '@/data/inmuebles/versiones-api';
import { etiquetaDeVersion, type FotoComparada, type RevisionFotos, type VersionDeFoto } from '@/domain/versiones';
import type { Inmueble } from '@/domain/inmueble';

interface Props {
  /* `null` = cerrado. */
  inmueble: Inmueble | null;
  alCerrar: () => void;
}

export function ModalRevisionVersiones({ inmueble, alCerrar }: Props) {
  const id = inmueble?.id ?? null;
  const [revision, setRevision] = useState<RevisionFotos | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    let vigente = true;
    setRevision(null);
    setError(null);
    leerRevisionFotos(id)
      .then((r) => {
        if (vigente) setRevision(r);
      })
      .catch((e: unknown) => {
        if (vigente) setError(e instanceof Error ? e.message : 'No se pudo leer la revisión.');
      });
    /* Al cambiar de inmueble (o cerrar) la respuesta vieja se descarta. */
    return () => {
      vigente = false;
    };
  }, [id]);

  return (
    <Dialog open={inmueble !== null} onOpenChange={(abierto) => !abierto && alCerrar()}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-5xl">
        <DialogHeader>
          <DialogTitle className="flex min-w-0 items-start gap-2">
            <Layers className="mt-0.5 h-4 w-4 shrink-0" />
            <span className="line-clamp-2">Versiones de las fotos: {inmueble?.titulo || 'Sin título'}</span>
          </DialogTitle>
          <DialogDescription>
            Cada columna es una foto y cada fila su original, su mejorada y las variantes. El % es cuánto se aparta
            del histograma de la original: poco % = demasiado parecida, y hay que mirarla a ojo.
          </DialogDescription>
        </DialogHeader>

        {error ? (
          <p role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </p>
        ) : null}

        {!revision && !error ? <p className="text-sm text-muted-foreground">Midiendo…</p> : null}
        {revision && revision.fotos.length === 0 ? (
          <p className="text-sm text-muted-foreground">El inmueble no tiene fotos todavía.</p>
        ) : null}

        {revision && revision.fotos.length > 0 ? (
          <div className="overflow-x-auto">
            <div className="flex min-w-max gap-2">
              {revision.fotos.map((foto) => (
                <ColumnaFoto key={foto.orden} foto={foto} />
              ))}
            </div>
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

/* Una foto con sus versiones en vertical: se comparan de arriba abajo sin
 * mover la vista a los lados. */
function ColumnaFoto({ foto }: { foto: FotoComparada }) {
  /* Hay fichas con más de una mejorada en el mismo hueco (subidas de antes):
   * las repetidas se numeran por orden de aparición para distinguirlas. */
  const etiquetas = etiquetarRepetidas(foto.versiones);
  return (
    <div className="grid w-48 shrink-0 content-start gap-1.5">
      <p className="text-xs font-medium text-muted-foreground">Foto {foto.orden + 1}</p>
      {foto.versiones.map((v, i) => (
        <CeldaVersion key={`${v.origen}-${v.version}-${i}`} version={v} etiqueta={etiquetas[i]} />
      ))}
      {foto.versiones.length === 1 ? (
        <p className="rounded-md border border-dashed px-2 py-1 text-[11px] text-muted-foreground">
          Sin mejorada ni variantes.
        </p>
      ) : null}
    </div>
  );
}

/* «Mejorada» → «Mejorada 1», «Mejorada 2» solo cuando hay dos en la columna. */
function etiquetarRepetidas(versiones: VersionDeFoto[]): string[] {
  const vistas = new Map<string, number>();
  for (const v of versiones) vistas.set(v.origen, (vistas.get(v.origen) ?? 0) + 1);
  const numero = new Map<string, number>();
  return versiones.map((v) => {
    const base = etiquetaDeVersion(v);
    if ((vistas.get(v.origen) ?? 0) < 2) return base;
    const n = (numero.get(v.origen) ?? 0) + 1;
    numero.set(v.origen, n);
    return `${base} ${n}`;
  });
}

function CeldaVersion({ version, etiqueta }: { version: VersionDeFoto; etiqueta: string }) {
  const pct = version.diferencia_pct;
  return (
    <figure className="overflow-hidden rounded-md border bg-background">
      {/* Miniatura que ya sirve el admin (`min160-`), no el original de 3 MB. */}
      <img
        src={miniatura(version.url)}
        alt={`${etiqueta} de la foto`}
        className="aspect-[4/3] w-full object-cover"
        loading="lazy"
      />
      <figcaption className="flex items-center justify-between gap-1 px-1.5 py-1">
        <span className="text-[11px] font-medium">{etiqueta}</span>
        <Diferencia pct={pct} />
      </figcaption>
    </figure>
  );
}

function Diferencia({ pct }: { pct: number | null }) {
  if (pct === null) return <span className="text-[11px] text-muted-foreground">—</span>;
  const redondeado = Math.round(pct);
  /* Menos de 8 % es «casi la misma foto»: lo que hay que rehacer. */
  const variante: 'destructive' | 'secondary' = redondeado < 8 ? 'destructive' : 'secondary';
  return (
    <Badge variant={variante} title={`${pct.toFixed(1)} % de diferencia con la original`}>
      {redondeado} %
    </Badge>
  );
}

/* El admin sirve `min160-<archivo>` junto a cada foto; si aún no existe, el
 * servidor la genera al vuelo y la devuelve igual. */
function miniatura(url: string): string {
  const corte = url.lastIndexOf('/');
  return corte < 0 ? url : `${url.slice(0, corte + 1)}min160-${url.slice(corte + 1)}`;
}
