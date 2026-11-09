import { Loader2, Save } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useConfigVersiones } from '@/hooks/configuracion/use-config-versiones';

/* Pestaña Versiones [30AA-1]: cuántas versiones tiene cada inmueble (1..5).
 * Es un ajuste GLOBAL: la v1 es siempre la web pública y cada versión extra
 * es una pestaña más en el modal, para publicar en otra cuenta de Facebook
 * sin repetir el texto. */
export function PestanaVersiones() {
  const { versiones, borrador, guardando, mensaje, guardar, cambiar } = useConfigVersiones();

  if (versiones === null) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        {mensaje ? mensaje : <><Loader2 className="h-4 w-4 animate-spin" /> Cargando…</>}
      </div>
    );
  }

  return (
    <div className="grid gap-3">
      <label className="grid gap-1 text-sm font-medium">
        Versiones por inmueble
        <span className="text-xs font-normal text-muted-foreground">
          La 1 es la ficha pública y no cambia nunca. Cada versión extra es una pestaña del modal con su propio
          título, su descripción y sus fotos variante, para publicar el mismo inmueble en otra cuenta de Facebook.
        </span>
        <Input
          type="number"
          min={1}
          max={5}
          value={borrador}
          onChange={(e) => cambiar(Number(e.target.value))}
        />
      </label>

      <div className="flex items-center gap-2">
        <Button size="sm" disabled={guardando || borrador === versiones} onClick={() => void guardar()}>
          {guardando ? <Loader2 className="animate-spin" /> : <Save />} Guardar
        </Button>
        {mensaje && <span className="text-xs text-muted-foreground">{mensaje}</span>}
        {borrador === versiones && !mensaje && (
          <span className="text-xs text-muted-foreground">Sin cambios</span>
        )}
      </div>
    </div>
  );
}
