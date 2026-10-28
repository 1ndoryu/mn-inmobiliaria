/* [28AA-6] Vista mensual: un chip por publicación en su día. Las salidas
 * (`publicada`) van en verde, las listas en ámbar y los borradores en gris. */
import type { Publicacion } from '@/domain/redes/publicacion-redes';
import { aIso } from '@/hooks/publicaciones/use-calendario';
import { cn } from '@/lib/utils';
import { agruparPorDia, CLASE_ESTADO, diasDeRejilla, ETIQUETA_ESTADO } from './calendario-util';

const DIAS_SEMANA = ['Lun', 'Mar', 'Mié', 'Jue', 'Vie', 'Sáb', 'Dom'];

interface RejillaMesProps {
  mes: Date;
  publicaciones: Publicacion[];
  /* Día de hoy como `YYYY-MM-DD`, para marcarlo. */
  hoy: string;
  alElegir: (publicacion: Publicacion) => void;
}

export function RejillaMes({ mes, publicaciones, hoy, alElegir }: RejillaMesProps) {
  const porDia = agruparPorDia(publicaciones);
  const dias = diasDeRejilla(mes);

  return (
    /* Scroll horizontal en móvil: siete columnas no caben en un teléfono. */
    <div className="overflow-x-auto rounded-lg border">
      <div className="min-w-[640px]">
        <div className="grid grid-cols-7 border-b bg-muted/50 text-xs font-medium text-muted-foreground">
          {DIAS_SEMANA.map((d) => (
            <div key={d} className="px-2 py-1.5 text-center">
              {d}
            </div>
          ))}
        </div>
        <div className="grid grid-cols-7">
          {dias.map((dia) => {
            const clave = aIso(dia);
            const delMes = dia.getMonth() === mes.getMonth();
            const esHoy = clave === hoy;
            const lista = porDia.get(clave) ?? [];
            return (
              <div
                key={clave}
                className={cn(
                  'min-h-24 border-b border-r p-1 text-xs',
                  !delMes && 'bg-muted/30 text-muted-foreground',
                  esHoy && 'bg-primary/5',
                )}
              >
                <span
                  className={cn(
                    'inline-block px-1',
                    esHoy && 'rounded bg-primary font-semibold text-primary-foreground',
                  )}
                >
                  {dia.getDate()}
                </span>
                <ul className="mt-1 flex flex-col gap-1">
                  {lista.map((p) => (
                    <li key={p.id}>
                      <button
                        type="button"
                        onClick={() => alElegir(p)}
                        title={`${p.tituloInmueble} · ${ETIQUETA_ESTADO[p.estado]}`}
                        className={cn(
                          'w-full truncate rounded border px-1 py-0.5 text-left',
                          CLASE_ESTADO[p.estado],
                        )}
                      >
                        {p.tituloInmueble}
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
