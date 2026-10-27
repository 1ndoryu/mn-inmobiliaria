import { Input } from '@/components/ui/input';
import { Select } from '@/components/ui/select';
import type { InmuebleDraft } from '@/domain/inmueble';
import { Etiqueta } from '../formulario/campos-formulario';

/* [25AA-7] Operación y, en «ambas», el canon de alquiler. Vive aquí y no en
 * `modal-inmueble` para no empujar ese componente por encima del tope de 300
 * líneas (regla `limite-lineas` de Sentinel). Fragmento: el canon ocupa su
 * propia celda de la rejilla del formulario. */
export function CampoOperacion({
  operacion,
  precioAlquiler,
  error,
  alCambiarOperacion,
  alCambiarCanon,
}: {
  operacion: InmuebleDraft['operacion'];
  precioAlquiler: string;
  error?: string;
  alCambiarOperacion: (valor: InmuebleDraft['operacion']) => void;
  alCambiarCanon: (valor: string) => void;
}) {
  return (
    <>
      <div className="space-y-1">
        <Etiqueta error={error}>Operación</Etiqueta>
        <Select
          value={operacion}
          onChange={(e) => alCambiarOperacion(e.target.value as InmuebleDraft['operacion'])}
          className={error ? 'border-destructive' : undefined}
        >
          <option value="">Seleccionar…</option>
          <option value="venta">venta</option>
          <option value="alquiler">alquiler</option>
          <option value="ambas">ambas (venta y alquiler)</option>
        </Select>
        {error && <p className="text-xs text-destructive">{error}</p>}
      </div>
      {operacion === 'ambas' && (
        <div className="space-y-1">
          <Etiqueta>Canon de alquiler ($/mes)</Etiqueta>
          <Input value={precioAlquiler} onChange={(e) => alCambiarCanon(e.target.value)} placeholder="2800" inputMode="numeric" />
        </div>
      )}
    </>
  );
}
