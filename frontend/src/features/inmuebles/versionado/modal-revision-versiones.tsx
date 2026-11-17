/* [30AA-14] Modal de revisión: todas las versiones de las fotos de un inmueble
 * y el % de diferencia de cada una contra su original. La medición la hace el
 * backend (`GET /inmuebles/:id/fotos/versiones`): el front solo pinta. Se abre
 * desde la acción «Revisar versiones» de la fila.
 *
 * Una foto por bloque, de arriba abajo, con sus versiones en fila: se recorre
 * la lista en vertical sin arrastrar una barra horizontal de ancho fijo. Al
 * pulsar una miniatura se abre la imagen completa.
 *
 * Vive en `versionado/` porque la carpeta `inmuebles/` está en su tope de 10
 * archivos (hallazgo del gate, 2026-11-16). */
import { useEffect, useRef, useState, type CSSProperties } from 'react';
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
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle className="flex min-w-0 items-start gap-2">
            <Layers className="mt-0.5 h-4 w-4 shrink-0" />
            <span className="line-clamp-2">Versiones de las fotos: {inmueble?.titulo || 'Sin título'}</span>
          </DialogTitle>
          <DialogDescription>
            Cada bloque es una foto, con su original y sus versiones en fila. El % es cuánto se aparta del histograma
            de la original: poco % = demasiado parecida, y hay que mirarla a ojo. Pulsa una imagen para verla entera.
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
          <div className="flex flex-col gap-4">
            {revision.fotos.map((foto) => (
              <FilaFoto key={foto.orden} foto={foto} />
            ))}
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

/* Una foto con sus versiones en fila, una debajo de otra. */
function FilaFoto({ foto }: { foto: FotoComparada }) {
  /* Hay fichas con más de una mejorada en el mismo hueco (subidas de antes):
   * las repetidas se numeran por orden de aparición para distinguirlas. */
  const etiquetas = etiquetarRepetidas(foto.versiones);
  /* Contra qué comparar al ampliar: la original de esta foto, si la hay. */
  const original = foto.versiones.find((v) => v.origen === 'original')?.url ?? null;
  return (
    <section className="rounded-lg border p-2">
      <p className="mb-1.5 px-1 text-xs font-medium text-muted-foreground">Foto {foto.orden + 1}</p>
      <div className="flex flex-wrap gap-2">
        {foto.versiones.map((v, i) => (
          <CeldaVersion
            key={`${v.origen}-${v.version}-${i}`}
            version={v}
            etiqueta={etiquetas[i]}
            original={v.origen === 'original' ? null : original}
          />
        ))}
        {foto.versiones.length === 1 ? (
          <p className="self-center rounded-md border border-dashed px-2 py-1 text-[11px] text-muted-foreground">
            Sin mejorada ni variantes.
          </p>
        ) : null}
      </div>
    </section>
  );
}

/* «Mejorada» → «Mejorada 1», «Mejorada 2» solo cuando hay dos en la foto. */
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

function CeldaVersion({
  version,
  etiqueta,
  original,
}: {
  version: VersionDeFoto;
  etiqueta: string;
  /* `null` en la propia original (no hay contra qué deslizar). */
  original: string | null;
}) {
  const [abierta, setAbierta] = useState(false);
  const pct = version.diferencia_pct;
  return (
    <figure className="w-44 shrink-0 overflow-hidden rounded-md border bg-background">
      {/* La miniatura es la que ya sirve el admin (`min160-`); al pulsarla se
       * abre el archivo completo. */}
      <button
        type="button"
        onClick={() => setAbierta(true)}
        aria-label={`Ver ${etiqueta} entera`}
        className="block w-full cursor-zoom-in focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30"
      >
        <img
          src={miniatura(version.url)}
          alt={`${etiqueta} de la foto`}
          className="aspect-[4/3] w-full object-cover"
          loading="lazy"
        />
      </button>
      <figcaption className="flex items-center justify-between gap-1 px-1.5 py-1">
        <span className="text-[11px] font-medium">{etiqueta}</span>
        <Diferencia pct={pct} />
      </figcaption>
      <Dialog open={abierta} onOpenChange={setAbierta}>
        <DialogContent className="max-h-[94dvh] overflow-auto sm:max-w-5xl">
          <DialogHeader>
            <DialogTitle className="text-sm">
              {etiqueta}
              {original ? ' · frente a la original' : ''}
            </DialogTitle>
          </DialogHeader>
          {original ? (
            <Comparador antes={original} despues={version.url} etiqueta={etiqueta} />
          ) : (
            /* `max-h` + `object-contain`: la foto entera sin recortar ni deformar. */
            <img src={version.url} alt={`${etiqueta} de la foto, tamaño completo`} className="max-h-[80dvh] w-full object-contain" />
          )}
        </DialogContent>
      </Dialog>
    </figure>
  );
}

/* Comparador de dos fotos: la original debajo y la versión encima, recortada
 * por una línea que se arrastra (o se mueve con las flechas del rango). Sirve
 * para ver qué cambió y dónde sin ir y volver entre dos imágenes separadas. */
function Comparador({ antes, despues, etiqueta }: { antes: string; despues: string; etiqueta: string }) {
  /* Dónde está la línea, en % del ancho: 0 = solo la versión, 100 = solo la
   * original. El arrastre y el rango escriben el mismo estado. */
  const [corte, setCorte] = useState(50);
  const marco = useRef<HTMLDivElement>(null);
  const arrastrando = useRef(false);

  function mover(clientX: number) {
    const caja = marco.current?.getBoundingClientRect();
    if (!caja || caja.width === 0) return;
    setCorte(Math.min(100, Math.max(0, ((clientX - caja.left) / caja.width) * 100)));
  }

  return (
    <div className="space-y-2">
      <div className="flex justify-center">
        <div
          ref={marco}
          className="relative inline-block max-w-full touch-none select-none overflow-hidden rounded-lg bg-muted"
          style={{ '--corte': `${corte}%` } as CSSProperties}
          onPointerDown={(e) => {
            arrastrando.current = true;
            marco.current?.setPointerCapture(e.pointerId);
            mover(e.clientX);
          }}
          onPointerMove={(e) => arrastrando.current && mover(e.clientX)}
          onPointerUp={() => (arrastrando.current = false)}
          onPointerCancel={() => (arrastrando.current = false)}
        >
          {/* La versión manda el tamaño; la original se recorta encima. */}
          <img src={despues} alt={etiqueta} draggable={false} className="block max-h-[76dvh] w-auto max-w-full object-contain" />
          <img
            src={antes}
            alt="Original"
            draggable={false}
            className="comparador-antes absolute inset-0 h-full w-full object-contain"
          />
          <div className="comparador-linea pointer-events-none absolute inset-y-0 w-0.5 -translate-x-1/2 bg-white shadow-[0_0_0_1px_rgba(0,0,0,0.35)]">
            <span className="absolute top-1/2 left-1/2 h-7 w-7 -translate-x-1/2 -translate-y-1/2 rounded-full bg-white text-center text-[13px] leading-7 text-black shadow">
              ↔
            </span>
          </div>
          <span className="pointer-events-none absolute top-2 left-2 rounded bg-black/60 px-1.5 py-0.5 text-[10px] text-white">Original</span>
          <span className="pointer-events-none absolute top-2 right-2 rounded bg-black/60 px-1.5 py-0.5 text-[10px] text-white">{etiqueta}</span>
        </div>
      </div>
      <input
        type="range"
        min={0}
        max={100}
        value={Math.round(corte)}
        onChange={(e) => setCorte(Number(e.target.value))}
        aria-label="Mover la línea de comparación"
        className="w-full"
      />
    </div>
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
