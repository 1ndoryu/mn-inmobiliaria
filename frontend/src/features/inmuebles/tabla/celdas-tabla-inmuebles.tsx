/* [08AA-13] Celdas presentacionales de la tabla de inmuebles.
 * Viven en `tabla/` (no en `tabla-inmuebles.tsx`) por el límite de 300 líneas
 * y para no abarrotar `features/inmuebles/` (máx 10 archivos). Sin cambio de
 * conducta ni de estilos. `claseEstadoDe` es función (no objeto exportado)
 * para no disparar `objeto-mutable-exportado`: el mapa vive dentro. */
import { Globe } from 'lucide-react';
import { estaVerificado, formatearPrecio, type EstadoInmueble, type Inmueble } from '@/domain/inmueble';
import { calcularCompletitud } from '@/domain/ficha-ask';
import { resumenVinculos } from '@/domain/versiones';
import { Badge, badgeVariants } from '@/components/ui/badge';
import { ButtonPlano } from '@/components/ui/button';
import { cn } from '@/lib/utils';

/* Color del badge de estado. Función con mapa interno: el llamador no puede
 * mutar el mapa compartido. */
export function claseEstadoDe(estado: EstadoInmueble): string {
  const mapa: Record<EstadoInmueble, string> = {
    disponible: 'border-transparent bg-emerald-100 text-emerald-900',
    reservado: 'border-transparent bg-amber-100 text-amber-900',
    vendido: 'border-transparent bg-sky-100 text-sky-900',
    alquilado: '',
  };
  return mapa[estado];
}

/* Sin obligatorios: lo no indicado se muestra neutro ("—" / "Sin título"). */
export function precioVisible(precio: number): string {
  return precio > 0 ? formatearPrecio(precio) : '—';
}

/* Semáforo de ficha /ask (279A-3): % de preguntas respondidas por tipo.
 * Solo aviso visual: nunca bloquea publicar ni editar. */
export function SemaforoFicha({ inmueble }: { inmueble: Inmueble }) {
  const { porcentaje } = calcularCompletitud(inmueble.tipo, inmueble.extras ?? {}, inmueble.precioMinimo ?? null, inmueble);
  const color =
    porcentaje === 100
      ? 'border-transparent bg-emerald-100 text-emerald-900'
      : porcentaje >= 50
        ? 'border-transparent bg-amber-100 text-amber-900'
        : 'border-transparent bg-red-100 text-red-900';
  return (
    <Badge variant="secondary" className={color} title={`Ficha al ${porcentaje}%`}>
      {porcentaje}%
    </Badge>
  );
}

/* Etiqueta de visibilidad en la web pública. */
export function Publico({ publicado }: { publicado: boolean }) {
  if (!publicado) return <span className="text-xs text-muted-foreground">Oculto</span>;
  return (
    <Badge variant="secondary" className="border-transparent bg-emerald-100 text-emerald-900">
      <Globe className="h-3 w-3" /> Público
    </Badge>
  );
}

/* [09AA-19 F7c] Badge de vínculo Marketplace: `verificado` con ficha exacta,
 * `sin vincular` sin ella. Fuente de verdad futura: `avisoConocido` del
 * backend (`aviso_conocido`); sin ella se deriva de `marketplaceId`.
 * [30AA-7] Cada versión se publica como aviso aparte: el backend manda
 * `faltanAvisos` y el badge dice cuántas versiones quedan sin vínculo.
 * [30AA-9] Con `alAbrir` el badge es un botón que abre el modal de vínculos
 * de esa ficha; sin él se pinta igual que siempre (presentacional).
 * Presentacional puro (la tabla no decide). */
export function VinculoBadge({ inmueble, alAbrir }: { inmueble: Inmueble; alAbrir?: () => void }) {
  const resumen = resumenVinculos(inmueble.faltanAvisos);
  const verificado = estaVerificado(inmueble);
  /* Se elige variante, clases y texto una vez; el envoltorio (span o botón)
   * se decide al final, para que ambos caminos no se separen. */
  let variante: 'secondary' | 'outline' = 'secondary';
  let clases = 'border-transparent bg-amber-100 text-amber-900';
  let texto = resumen?.etiqueta ?? '';
  let titulo = '';

  if (resumen?.incompleto) {
    /* Hay versiones sin aviso: se nombran en el `title` para no ensanchar la
     * columna. Las versiones cubiertas no se listan (la API las omite).
     * Ámbar sólido, como el resto de badges de la tabla: el translúcido con
     * borde desentonaba en la fila. */
    titulo = `Sin aviso en ninguna cuenta: ${resumen.faltan.map((v) => `versión ${v}`).join(', ')}`;
  } else if (verificado) {
    clases = 'border-transparent bg-emerald-100 text-emerald-900';
    texto = resumen ? 'al día' : 'verificado';
    titulo = inmueble.marketplaceId ? `Aviso ${inmueble.marketplaceId}` : 'Aviso conocido por el backend';
  } else {
    variante = 'outline';
    clases = 'text-muted-foreground';
    texto = 'sin vincular';
    titulo = 'Sin ID de aviso: el borrador usa título aproximado';
  }

  if (alAbrir) {
    /* Mismas clases que el `Badge`, en un botón: se puede pulsar con teclado
     * y el modal se abre desde la fila sin entrar a editar el inmueble. */
    return (
      <ButtonPlano
        type="button"
        onClick={alAbrir}
        title={`${titulo}. Pulsa para configurar los vínculos.`}
        aria-label={`Configurar vínculos de ${inmueble.titulo || 'Sin título'}: ${texto}`}
        className={cn(
          badgeVariants({ variant: variante }),
          clases,
          'cursor-pointer hover:brightness-95 focus-visible:ring-[3px] focus-visible:ring-ring/50',
        )}
      >
        {texto}
      </ButtonPlano>
    );
  }

  return (
    <Badge variant={variante} className={clases} title={titulo}>
      {texto}
    </Badge>
  );
}
