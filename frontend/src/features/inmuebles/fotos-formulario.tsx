import { ImagePlus, Star, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';

/* Galería del formulario de inmueble: miniaturas con quitar una a una,
 * elección de principal (la primera es la que muestra la web pública) +
 * botón de añadir. Extraído de `modal-inmueble` (Sentinel limite-lineas).
 * Sentinel (key-index-lista): la clave combina índice + firma del dataURL
 * en vez del índice solo, para no reconciliar mal al quitar intermedias. */
export function FotosFormulario({
  fotos,
  aviso,
  alElegir,
  alQuitar,
  alHacerPrincipal,
  inputRef,
  alSeleccionar,
}: {
  fotos: string[];
  aviso: string | null;
  alElegir: () => void;
  alQuitar: (indice: number) => void;
  alHacerPrincipal: (indice: number) => void;
  inputRef: React.RefObject<HTMLInputElement | null>;
  alSeleccionar: (files: FileList | null) => void;
}) {
  return (
    <div className="space-y-2 sm:col-span-2">
      <span className="block text-sm font-medium">Fotos ({fotos.length})</span>
      <p className="text-xs text-muted-foreground">La primera es la principal en la web pública.</p>
      <input
        ref={inputRef}
        type="file"
        accept="image/*"
        multiple
        className="hidden"
        onChange={(e) => alSeleccionar(e.target.files)}
      />
      {fotos.length > 0 && (
        <div className="grid grid-cols-3 gap-2">
          {fotos.map((f, idx) => (
            <div key={`${idx}-${f.length}-${f.slice(-16)}`} className="group relative">
              <img src={f} alt={`Foto ${idx + 1}`} className="h-24 w-full rounded-md object-cover" />
              {idx === 0 ? (
                <Badge className="absolute top-1 left-1 flex items-center gap-1 text-[10px]">
                  <Star className="h-3 w-3" /> Principal
                </Badge>
              ) : (
                <button
                  type="button"
                  title="Elegir como principal"
                  aria-label={`Elegir foto ${idx + 1} como principal`}
                  onClick={() => alHacerPrincipal(idx)}
                  className="absolute top-1 left-1 rounded-full bg-black/70 p-1 text-white hover:bg-black"
                >
                  <Star className="h-3.5 w-3.5" />
                </button>
              )}
              <button
                type="button"
                title="Quitar foto"
                onClick={() => alQuitar(idx)}
                className="absolute top-1 right-1 rounded-full bg-black/70 p-1 text-white hover:bg-black"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          ))}
        </div>
      )}
      <Button type="button" variant="outline" onClick={alElegir}>
        <ImagePlus /> Añadir fotos
      </Button>
      {aviso && <p className="text-xs text-amber-700">{aviso}</p>}
    </div>
  );
}
