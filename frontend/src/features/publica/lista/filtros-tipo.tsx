import {
  Building,
  Building2,
  House,
  LandPlot,
  LayoutGrid,
  Store,
  type LucideIcon,
} from 'lucide-react';
import { ETIQUETAS_TIPO, TIPOS, type InmueblePublico, type TipoInmueble } from '../../../domain/inmueble';
import { CLASE_ACTIVO, CLASE_BORDE, CLASE_REPOSO, CLASE_TEXTO } from '../disenno';

const ICONOS_TIPO: Record<TipoInmueble, LucideIcon> = {
  apartamento: Building2,
  casa: House,
  local: Store,
  terreno: LandPlot,
  townhouse: Building,
};

function BotonFiltro({
  activo,
  alElegir,
  icono: Icono,
  etiqueta,
  total,
}: {
  activo: boolean;
  alElegir: () => void;
  icono: LucideIcon;
  etiqueta: string;
  total: number;
}) {
  return (
    <button
      type="button"
      onClick={alElegir}
      aria-pressed={activo}
      className={`flex min-w-0 flex-1 cursor-pointer items-center justify-center gap-2 rounded-none border-0 px-4 py-3 ${CLASE_TEXTO} whitespace-nowrap ${
        activo ? CLASE_ACTIVO : CLASE_REPOSO
      }`}
    >
      <Icono className="h-4 w-4" />
      {etiqueta} ({total})
    </button>
  );
}

export function FiltrosTipo({
  filtro,
  elegir,
  inmuebles,
}: {
  filtro: TipoInmueble | null;
  elegir: (t: TipoInmueble | null) => void;
  inmuebles: InmueblePublico[];
}) {
  // TEMPORAL: se muestran todos los tipos para previsualizar (aunque tengan 0).
  const tiposPresentes = TIPOS;
  return (
    <div className={`flex w-full gap-1 overflow-x-auto border ${CLASE_BORDE}`} role="group" aria-label="Filtrar por tipo de propiedad">
      <BotonFiltro
        activo={filtro === null}
        alElegir={() => elegir(null)}
        icono={LayoutGrid}
        etiqueta="Todos"
        total={inmuebles.length}
      />
      {tiposPresentes.map((t) => (
        <BotonFiltro
          key={t}
          activo={filtro === t}
          alElegir={() => elegir(filtro === t ? null : t)}
          icono={ICONOS_TIPO[t]}
          etiqueta={ETIQUETAS_TIPO[t]}
          total={inmuebles.filter((i) => i.tipo === t).length}
        />
      ))}
    </div>
  );
}
