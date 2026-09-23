import { BedDouble, Bath, Building2, CalendarPlus, CalendarClock, CarFront, Download, EyeOff, Globe, LandPlot, Loader2, MapPin, Pencil, Ruler, Sparkles } from 'lucide-react';
import {
  ETIQUETAS_TIPO,
  formatearFecha,
  formatearPrecio,
  type EstadoInmueble,
  type Inmueble,
} from '@/domain/inmueble';
import type { FotoMejora } from '@/domain/foto-mejora';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { cn } from '@/lib/utils';
import { useModalVerInmueble } from '@/hooks/inmuebles/use-modal-ver-inmueble';

const claseEstado: Record<EstadoInmueble, string> = {
  disponible: 'border-transparent bg-emerald-100 text-emerald-900',
  reservado: 'border-transparent bg-amber-100 text-amber-900',
  vendido: 'border-transparent bg-sky-100 text-sky-900',
  alquilado: '',
};

interface Props {
  inmueble: Inmueble | null;
  /** Todas las fotos de mejora (se filtran por inmueble+orden). */
  fotosMejora: FotoMejora[];
  alCambiarAbierto: (abierto: boolean) => void;
  onEditar: (inmueble: Inmueble) => void;
  onPublicar: (id: string, publicado: boolean) => void;
}

export function ModalVerInmueble({ inmueble, fotosMejora, alCambiarAbierto, onEditar, onPublicar }: Props) {
  const {
    indice,
    setIndice,
    descargando,
    publicando,
    errorDescarga,
    mejoradaDe,
    mejoradaActual,
    descargarTodas,
    cambiarPublicacion,
  } = useModalVerInmueble(inmueble, fotosMejora, onPublicar);

  return (
    <Dialog open={inmueble !== null} onOpenChange={alCambiarAbierto}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-2xl">
        {inmueble && (
          <>
            <DialogHeader>
              <div className="flex items-start justify-between gap-2">
                <div>
                  <DialogTitle className={cn(!inmueble.titulo && 'text-muted-foreground')}>
                    {inmueble.titulo || 'Sin título'}
                  </DialogTitle>
                  <DialogDescription className="flex items-center gap-1">
                    <MapPin className="h-3.5 w-3.5" /> {[inmueble.ubicacion, inmueble.residencia].filter(Boolean).join(' · ') || '—'}
                  </DialogDescription>
                </div>
                <Badge variant="secondary" className={cn('shrink-0 capitalize', claseEstado[inmueble.estado])}>
                  {inmueble.estado}
                </Badge>
                <Badge
                  variant={inmueble.publicado ? 'default' : 'outline'}
                  className="shrink-0"
                  title={inmueble.publicado ? 'Visible en la web pública' : 'Oculto en la web pública'}
                >
                  {inmueble.publicado ? 'Publicado' : 'No publicado'}
                </Badge>
              </div>
            </DialogHeader>

            {inmueble.fotos.length > 0 && indice >= 0 ? (
              <div className="space-y-2">
                <div className="flex items-center justify-between gap-2">
                  <p className="text-xs text-muted-foreground">
                    {mejoradaActual ? 'Mostrando versión mejorada.' : 'Original (aún sin mejorar).'}
                  </p>
                  <Button variant="outline" size="sm" onClick={descargarTodas} disabled={descargando}>
                    {descargando ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Download className="h-3.5 w-3.5" />}
                    Descargar todas ({inmueble.fotos.length})
                  </Button>
                </div>
                {errorDescarga && <p className="text-xs text-destructive">{errorDescarga}</p>}
                <div className="relative">
                  <img
                    src={mejoradaActual ?? inmueble.fotos[indice]}
                    alt={`${inmueble.titulo || 'Inmueble'} — foto ${indice + 1}`}
                    className="max-h-[50dvh] w-full rounded-md object-contain bg-muted"
                    onError={() => setIndice(-1)}
                  />
                  {mejoradaActual && (
                    <Badge className="absolute left-2 top-2 flex items-center gap-1" title="Versión mejorada por IA">
                      <Sparkles className="h-3 w-3" /> Mejorada
                    </Badge>
                  )}
                </div>
                {inmueble.fotos.length > 1 && (
                  <div className="grid grid-cols-6 gap-2">
                    {inmueble.fotos.map((f, i) => (
                      <button
                        key={f.slice(-32) + i}
                        type="button"
                        onClick={() => setIndice(i)}
                        aria-label={`Ver foto ${i + 1}`}
                        className={cn(
                          'relative overflow-hidden rounded-md border-2',
                          i === indice ? 'border-primary' : 'border-transparent',
                        )}
                      >
                        <img
                          src={f}
                          alt=""
                          className="aspect-square w-full object-cover"
                          onError={(e) => {
                            (e.target as HTMLImageElement).style.display = 'none';
                          }}
                        />
                        {mejoradaDe(i) && (
                          <span
                            className="absolute left-1 top-1 rounded-full bg-primary p-1 text-primary-foreground"
                            title="Tiene versión mejorada"
                          >
                            <Sparkles className="h-3 w-3" />
                          </span>
                        )}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <div className="flex h-40 items-center justify-center rounded-md bg-muted">
                <Building2 className="h-10 w-10 text-muted-foreground" />
              </div>
            )}

            <dl className="grid gap-2 text-sm sm:grid-cols-2">
              <div className="rounded-md border p-3">
                <dt className="text-muted-foreground">Precio</dt>
                <dd className="text-lg font-bold">{inmueble.precio > 0 ? formatearPrecio(inmueble.precio) : '—'}</dd>
              </div>
              <div className="rounded-md border p-3">
                <dt className="text-muted-foreground">Tipo / Operación</dt>
                <dd className="font-medium capitalize">
                  {ETIQUETAS_TIPO[inmueble.tipo]} · {inmueble.operacion}
                </dd>
              </div>
              <div className="flex items-center gap-4 rounded-md border p-3 sm:col-span-2">
                {inmueble.habitaciones > 0 && (
                  <span className="flex items-center gap-1">
                    <BedDouble className="h-4 w-4 text-muted-foreground" /> {inmueble.habitaciones} hab
                  </span>
                )}
                {inmueble.banos > 0 && (
                  <span className="flex items-center gap-1">
                    <Bath className="h-4 w-4 text-muted-foreground" /> {inmueble.banos} baños
                  </span>
                )}
                {inmueble.metros > 0 && (
                  <span className="flex items-center gap-1" title="M² propiedad (construidos)">
                    <Ruler className="h-4 w-4 text-muted-foreground" /> {inmueble.metros} m²
                  </span>
                )}
                {inmueble.metrosTerreno > 0 && (
                  <span className="flex items-center gap-1" title="M² terreno">
                    <LandPlot className="h-4 w-4 text-muted-foreground" /> {inmueble.metrosTerreno} m² terreno
                  </span>
                )}
                {inmueble.puestos > 0 && (
                  <span className="flex items-center gap-1" title="Puestos de estacionamiento">
                    <CarFront className="h-4 w-4 text-muted-foreground" /> {inmueble.puestos} puestos
                  </span>
                )}
                {inmueble.habitaciones === 0 && inmueble.banos === 0 && inmueble.metros === 0 && inmueble.metrosTerreno === 0 && inmueble.puestos === 0 && (
                  <span className="text-muted-foreground">—</span>
                )}
              </div>
              {inmueble.descripcion && (
                <div className="rounded-md border p-3 sm:col-span-2">
                  <dt className="text-muted-foreground">Descripción</dt>
                  <dd className="whitespace-pre-wrap">{inmueble.descripcion}</dd>
                </div>
              )}
              <div className="rounded-md border p-3 text-xs text-muted-foreground sm:col-span-2">
                <p className="flex items-center gap-1">
                  <CalendarPlus className="h-3.5 w-3.5" /> Creado el {formatearFecha(inmueble.createdAt)}
                </p>
                <p className="mt-1 flex items-center gap-1">
                  <CalendarClock className="h-3.5 w-3.5" /> Modificado el {formatearFecha(inmueble.updatedAt)}
                </p>
              </div>
            </dl>

            <DialogFooter>
              <Button variant="outline" onClick={() => alCambiarAbierto(false)}>
                Cerrar
              </Button>
              <Button variant="outline" onClick={() => void cambiarPublicacion()} disabled={publicando}>
                {publicando ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : inmueble.publicado ? (
                  <EyeOff className="h-4 w-4" />
                ) : (
                  <Globe className="h-4 w-4" />
                )}
                {inmueble.publicado ? 'Retirar' : 'Publicar'}
              </Button>
              <Button
                onClick={() => {
                  onEditar(inmueble);
                }}
              >
                <Pencil /> Editar
              </Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
