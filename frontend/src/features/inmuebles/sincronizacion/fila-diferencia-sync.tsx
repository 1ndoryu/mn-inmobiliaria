/* [13AA-12 · 25AA-2] Las tarjetas de «Resolver diferencias con el servidor»: una por
 * campo, y una sola para todas las fotos (el plan compara por posición, así que
 * mover la principal cambia muchas filas que son un único cambio de orden).
 * Solo dibujan; la elección vive en `useModalDiferenciasSync`. */
import type { FilaPlanSync } from '@/data/inmuebles/sync';
import type { Inmueble } from '@/domain/inmueble';
import { eleccionDeGrupo, resumirFotos } from '@/domain/sync/resumen-fotos';
import { nombreClave } from '@/domain/sync/sync-inmueble';
import { ColumnaLado, describirDecision, describirOrigen, etiquetaColumna } from './piezas-diferencias-sync';
import { Button } from '@/components/ui/button';

type AlElegir = (claves: string | string[], conservar: boolean) => void;

/* «Lo último» / «Conservar lo viejo» para una clave o para un grupo; `viejo` null = mezcla. */
function BotonesEleccion({
  etiqueta,
  viejo,
  alElegir,
}: {
  etiqueta: string;
  viejo: boolean | null;
  alElegir: (conservar: boolean) => void;
}) {
  return (
    <div role="group" aria-label={etiqueta} className="flex gap-1.5">
      <Button
        type="button"
        size="sm"
        variant={viejo === false ? 'default' : 'outline'}
        aria-pressed={viejo === false}
        onClick={() => alElegir(false)}
      >
        Lo último
      </Button>
      <Button
        type="button"
        size="sm"
        variant={viejo === true ? 'default' : 'outline'}
        aria-pressed={viejo === true}
        onClick={() => alElegir(true)}
      >
        Conservar lo viejo
      </Button>
    </div>
  );
}

export function FilaDiferencia({
  fila,
  viejo,
  inmueble,
  conFotos,
  alElegir,
}: {
  fila: FilaPlanSync;
  viejo: boolean;
  inmueble: Inmueble;
  /* La receta se dibuja con las fotos de Local: si las fotos difieren, hay que decirlo. */
  conFotos: boolean;
  alElegir: AlElegir;
}) {
  const nombre = nombreClave(fila.clave);
  return (
    <li className="space-y-2 rounded-lg border p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div>
          <p className="font-medium">{nombre}</p>
          <p className="text-xs text-muted-foreground">{describirOrigen(fila.lado)}</p>
        </div>
        <BotonesEleccion
          etiqueta={`Valor que queda en ${nombre}`}
          viejo={viejo}
          alElegir={(conservar) => alElegir(fila.clave, conservar)}
        />
      </div>
      <p className="text-sm">{describirDecision(fila, viejo)}</p>
      <div className="grid grid-cols-2 gap-3">
        {(['local', 'servidor'] as const).map((lado) => (
          <ColumnaLado
            key={lado}
            nombre={lado === 'local' ? 'Local' : 'Servidor'}
            clave={fila.clave}
            valor={fila[lado]}
            miniatura={lado === 'local' ? fila.miniatura_local : fila.miniatura_servidor}
            inmueble={inmueble}
            etiqueta={etiquetaColumna(fila, lado)}
            queda={fila.elegido === lado}
          />
        ))}
      </div>
      {fila.clave === 'receta' && conFotos && (
        <p className="text-xs text-muted-foreground">
          Las dos vistas se dibujan con las fotos de Local. Como las fotos también difieren, el servidor puede estar
          numerando otras.
        </p>
      )}
    </li>
  );
}

/* Todas las filas `foto:*` en una tarjeta: resumen en palabras, una elección para el
 * grupo y el detalle posición por posición plegado (donde cada una sigue eligiéndose). */
export function GrupoFotos({
  filas,
  conservarViejo,
  inmueble,
  alElegir,
}: {
  filas: FilaPlanSync[];
  conservarViejo: string[];
  inmueble: Inmueble;
  alElegir: AlElegir;
}) {
  const claves = filas.map((f) => f.clave);
  const eleccion = eleccionDeGrupo(claves, conservarViejo);
  return (
    <li className="space-y-2 rounded-lg border p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div>
          <p className="font-medium">Fotos (orden y principal)</p>
          <p className="text-xs text-muted-foreground">
            Un solo cambio: las fotos se comparan por posición, por eso mover la principal toca varias filas.
          </p>
        </div>
        <BotonesEleccion
          etiqueta="Valor que queda en las fotos"
          viejo={eleccion === 'mixto' ? null : eleccion === 'viejo'}
          alElegir={(conservar) => alElegir(claves, conservar)}
        />
      </div>
      <ul className="list-disc space-y-0.5 pl-5 text-sm">
        {resumirFotos(filas).map((linea) => (
          <li key={linea}>{linea}</li>
        ))}
      </ul>
      {eleccion === 'mixto' && (
        <p className="text-xs text-muted-foreground">
          Hay posiciones con «lo último» y otras con «lo viejo»: elige arriba para unificarlas.
        </p>
      )}
      <details className="text-sm">
        <summary className="cursor-pointer select-none text-muted-foreground">
          Ver las {filas.length} posiciones una por una
        </summary>
        <ul className="mt-2 space-y-2">
          {filas.map((fila) => (
            <FilaDiferencia
              key={fila.clave}
              fila={fila}
              viejo={conservarViejo.includes(fila.clave)}
              inmueble={inmueble}
              conFotos
              alElegir={alElegir}
            />
          ))}
        </ul>
      </details>
    </li>
  );
}
