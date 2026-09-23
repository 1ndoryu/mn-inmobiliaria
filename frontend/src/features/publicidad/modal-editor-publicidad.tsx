import { useState } from 'react';
import { HousePlus, Pencil } from 'lucide-react';
import {
  PRESETS_EXPORTACION,
  lineaTitulo1De,
  resolverReceta,
  tituloZonaDe,
  type ComposicionPublicidad,
  type FormatoPublicidad,
  type RecetaPublicidad,
} from '@/domain/plantilla-publicidad';
import { fotosVisiblesDe, type Inmueble } from '@/domain/inmueble';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { VistaPrevia } from './vista-publicidad';

interface Props {
  inmueble: Inmueble | null;
  receta: RecetaPublicidad | null;
  /* Puede ser asíncrono (guardado en el servidor): el botón muestra
   * "Guardando…" y el modal solo cierra si resuelve. */
  alGuardar: (receta: RecetaPublicidad) => void | Promise<unknown>;
  alCerrar: () => void;
  /* Atajo al editor de la propiedad (título, ubicación, fotos…): al volver,
   * la vista previa se dibuja con los datos frescos. */
  alEditarPropiedad?: (inmueble: Inmueble) => void;
}

type Rol = 'fondo' | 'circularGrande' | 'circularMediano';

const ROLES: Array<{ rol: Rol; clave: 'fondoIdx' | 'circularGrandeIdx' | 'circularMedianoIdx'; etiqueta: string }> = [
  { rol: 'fondo', clave: 'fondoIdx', etiqueta: 'Fondo' },
  { rol: 'circularGrande', clave: 'circularGrandeIdx', etiqueta: 'Círculo grande' },
  { rol: 'circularMediano', clave: 'circularMedianoIdx', etiqueta: 'Círculo mediano' },
];

/* Editor de la imagen publicitaria: elige las 3 fotos por posición y el
 * formato, con vista previa en vivo. La receta guarda ÍNDICES (no URLs)
 * para que sobreviva a subir/quitar fotos; al guardar queda para la
 * propiedad y los botones muestran lo que la imagen lleva puesto. Los
 * inputs de título arrancan con el texto efectivo (el que se ve en la
 * imagen), no con el campo vacío: si no se tocan, el título sigue
 * automático; si se editan, quedan personalizados. */
export function ModalEditorPublicidad({ inmueble, receta, alGuardar, alCerrar, alEditarPropiedad }: Props) {
  const [borrador, setBorrador] = useState<RecetaPublicidad | null>(null);
  const [guardando, setGuardando] = useState(false);
  // Texto tecleado (null = sin tocar: vale el efectivo, auto o personalizado).
  const [texto1, setTexto1] = useState<string | null>(null);
  const [texto2, setTexto2] = useState<string | null>(null);
  const abierto = inmueble !== null && receta !== null;
  // La receta vigente es el borrador si se está editando, si no la guardada.
  const vigente = borrador ?? receta;
  const candidatas = inmueble ? fotosVisiblesDe(inmueble) : [];

  // Texto efectivo = personalizado si lo hay, si no el automático.
  const auto1 = inmueble ? lineaTitulo1De(inmueble) : '';
  const auto2 = inmueble ? `en ${tituloZonaDe(inmueble)}` : '';
  const valor1 = texto1 ?? vigente?.titulo1?.trim() ?? '';
  const valor2 = texto2 ?? vigente?.titulo2?.trim() ?? '';
  const efectivo1 = valor1 || auto1;
  const efectivo2 = valor2 || auto2;
  // Lo que dibuja la vista previa: la receta con el texto en edición
  // (sin tocar, los valores guardados: el automático sigue automático y
  // conserva la zona en naranja, igual que la imagen final).
  const vista: ComposicionPublicidad | null =
    vigente && inmueble
      ? resolverReceta(inmueble, {
          ...vigente,
          titulo1: texto1 ?? vigente.titulo1 ?? '',
          titulo2: texto2 ?? vigente.titulo2 ?? '',
        })
      : null;

  const elegir = (clave: (typeof ROLES)[number]['clave'], idx: number) => {
    if (!vigente) return;
    setBorrador({ ...vigente, [clave]: idx });
  };

  const cambiarFormato = (formato: FormatoPublicidad) => {
    if (!vigente) return;
    setBorrador({ ...vigente, formato });
  };

  const cambiarPrecio = (conPrecio: boolean) => {
    if (!vigente) return;
    setBorrador({ ...vigente, conPrecio });
  };

  const cambiarTitulo = (campo: 'texto1' | 'texto2', valor: string) => {
    if (campo === 'texto1') setTexto1(valor);
    else setTexto2(valor);
  };

  const tituloAutomatico = () => {
    if (!vigente) return;
    setTexto1(null);
    setTexto2(null);
    setBorrador({ ...vigente, titulo1: '', titulo2: '' });
  };

  const cerrar = (abrir: boolean) => {
    if (!abrir) {
      setBorrador(null);
      setTexto1(null);
      setTexto2(null);
      alCerrar();
    }
  };

  const guardar = () => {
    // Sin tocar el texto se conserva lo guardado (automático sigue auto).
    if (!vigente || guardando) return;
    setGuardando(true);
    void Promise.resolve(
      alGuardar({ ...vigente, titulo1: texto1 ?? vigente.titulo1 ?? '', titulo2: texto2 ?? vigente.titulo2 ?? '' }),
    ).finally(() => setGuardando(false));
  };

  return (
    <Dialog open={abierto} onOpenChange={cerrar}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Pencil className="h-4 w-4" /> Editar publicidad
          </DialogTitle>
          <DialogDescription>
            Elige las 3 fotos de la imagen y el formato. Se guarda para esta propiedad.
          </DialogDescription>
        </DialogHeader>
        {inmueble && vigente ? (
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="overflow-hidden rounded-lg border">
              {inmueble && vista ? <VistaPrevia inmueble={inmueble} comp={vista} /> : null}
            </div>
            <div className="space-y-4">
              {ROLES.map(({ clave, etiqueta }) => (
                <div key={clave} className="space-y-1.5">
                  <p className="text-xs font-medium">{etiqueta}</p>
                  <div className="flex flex-wrap gap-1.5">
                    {candidatas.map((url, idx) => (
                      <button
                        key={url}
                        type="button"
                        onClick={() => elegir(clave, idx)}
                        className={`h-12 w-12 overflow-hidden rounded-md border-2 ${
                          vigente[clave] === idx ? 'border-primary' : 'border-transparent opacity-60 hover:opacity-100'
                        }`}
                        title={`${etiqueta} · foto ${idx + 1}`}
                      >
                        <img src={url} alt="" className="h-full w-full object-cover" />
                      </button>
                    ))}
                  </div>
                </div>
              ))}
              <div className="space-y-1.5">
                <p className="text-xs font-medium">Formato</p>
                <div className="flex gap-2">
                  {PRESETS_EXPORTACION.map((p) => (
                    <Button
                      key={p.formato}
                      variant={vigente.formato === p.formato ? 'default' : 'outline'}
                      size="sm"
                      className="h-7 text-xs"
                      onClick={() => cambiarFormato(p.formato)}
                    >
                      {p.etiqueta}
                    </Button>
                  ))}
                </div>
              </div>
              <div className="space-y-1.5">
                <p className="text-xs font-medium">Precio</p>
                <div className="flex gap-2">
                  <Button
                    variant={(vigente.conPrecio ?? true) ? 'default' : 'outline'}
                    size="sm"
                    className="h-7 text-xs"
                    onClick={() => cambiarPrecio(true)}
                  >
                    Con precio
                  </Button>
                  <Button
                    variant={(vigente.conPrecio ?? true) ? 'outline' : 'default'}
                    size="sm"
                    className="h-7 text-xs"
                    onClick={() => cambiarPrecio(false)}
                  >
                    Sin precio
                  </Button>
                </div>
              </div>
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <p className="text-xs font-medium">Título</p>
                  <Button variant="ghost" size="sm" className="h-6 text-xs" onClick={tituloAutomatico}>
                    Automático
                  </Button>
                </div>
                <Input
                  value={efectivo1}
                  placeholder={lineaTitulo1De(inmueble)}
                  onChange={(e) => cambiarTitulo('texto1', e.target.value)}
                  className="h-8 text-xs"
                  aria-label="Título línea 1"
                />
                <Input
                  value={efectivo2}
                  placeholder={`en ${tituloZonaDe(inmueble)}`}
                  onChange={(e) => cambiarTitulo('texto2', e.target.value)}
                  className="h-8 text-xs"
                  aria-label="Título línea 2"
                />
              </div>
            </div>
          </div>
        ) : null}
        <DialogFooter>
          {inmueble && alEditarPropiedad && (
            <Button variant="outline" className="mr-auto" onClick={() => alEditarPropiedad(inmueble)}>
              <HousePlus className="h-4 w-4" /> Editar propiedad
            </Button>
          )}
          <Button variant="outline" onClick={() => cerrar(false)}>
            Cancelar
          </Button>
          <Button disabled={!vigente || guardando} onClick={guardar}>
            {guardando ? 'Guardando…' : 'Guardar'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
