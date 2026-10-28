/* [28AA-6] Checklist de los cuatro destinos. Con los cuatro marcados, al guardar
 * la publicación pasa a `publicada`; si falta alguno, queda `lista`. */
import {
  DESTINOS_PUBLICACION,
  destinosPendientes,
  ETIQUETA_DESTINO,
  type Destinos,
} from '@/domain/redes/publicacion-redes';

interface ChecklistDestinosProps {
  destinos: Destinos;
  deshabilitado: boolean;
  alCambiar: (destinos: Destinos) => void;
}

export function ChecklistDestinos({ destinos, deshabilitado, alCambiar }: ChecklistDestinosProps) {
  const pendientes = destinosPendientes(destinos);
  return (
    <fieldset className="flex flex-col gap-2" disabled={deshabilitado}>
      <legend className="text-sm font-medium">Destinos</legend>
      <div className="grid gap-2 sm:grid-cols-2">
        {DESTINOS_PUBLICACION.map((k) => (
          <label key={k} className="flex items-center gap-2 rounded-md border px-3 py-2 text-sm">
            <input
              type="checkbox"
              className="size-4 accent-primary"
              checked={destinos[k]}
              onChange={(e) => alCambiar({ ...destinos, [k]: e.target.checked })}
            />
            {ETIQUETA_DESTINO[k]}
          </label>
        ))}
      </div>
      <p className="text-xs text-muted-foreground">
        {pendientes === 0 ? 'Los cuatro marcados: al guardar pasa a publicada.' : `Faltan ${pendientes} de 4.`}
      </p>
    </fieldset>
  );
}
