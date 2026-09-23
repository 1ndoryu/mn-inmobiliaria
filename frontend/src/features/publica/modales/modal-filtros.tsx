import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { ETIQUETAS_TIPO, TIPOS, type TipoInmueble } from '../../../domain/inmueble';
import type { useFiltrosAvanzados } from '../../../hooks/publica/use-filtros-avanzados';
import type { FiltrosAvanzados } from '../busqueda';
import { CLASE_ACTIVO, CLASE_BORDE, CLASE_FONDO, CLASE_TEXTO, CLASE_TINTA, SOLO_MOVIL_TABLETA } from '../disenno';

/* Modal "Filtros avanzados": mismo concepto que el modal publicar (cuadrado,
 * tinta, sin sombras, Söhne 400 sin negritas; cierra con overlay o Escape,
 * sin X, y el borrador no se pierde al cerrar). Aplica operación, rango de
 * precio, rango de metros, ubicación y habitaciones mínimas. En móvil
 * (<md) incluye además el tipo de inmueble en un select, porque la fila
 * de filtros por tipo se oculta en ese tramo; el select aplica al acto,
 * igual que los botones de escritorio. */
export function ModalFiltros({
  modal,
  alAplicar,
  tipo,
  alElegirTipo,
}: {
  modal: ReturnType<typeof useFiltrosAvanzados>;
  alAplicar: () => void;
  tipo: TipoInmueble | null;
  alElegirTipo: (t: TipoInmueble | null) => void;
}) {
  return (
    <Dialog open={modal.abierto} onOpenChange={(a) => !a && modal.cerrar()}>
      <DialogContent
        showCloseButton={false}
        className={`max-h-[92dvh] overflow-y-auto rounded-none border ${CLASE_BORDE} ${CLASE_FONDO} ${CLASE_TINTA} p-6 font-soehne font-normal sm:max-w-2xl`}
      >
        <DialogTitle className="font-soehne text-xl leading-tight font-normal">
          Filtros avanzados
        </DialogTitle>
        <div className="mt-4 flex flex-col gap-4 font-normal">
          <TipoFiltro valor={tipo} alElegir={alElegirTipo} />
          <OperacionFiltro valor={modal.borrador.operacion} alElegir={(v) => modal.actualizar('operacion', v)} />
          <span className="flex gap-2">
            <Campo
              etiqueta="Precio mín. (USD)"
              valor={modal.borrador.precioMin}
              alCambiar={(v) => modal.actualizar('precioMin', v)}
              placeholder="50000"
              ancho="flex-1"
            />
            <Campo
              etiqueta="Precio máx. (USD)"
              valor={modal.borrador.precioMax}
              alCambiar={(v) => modal.actualizar('precioMax', v)}
              placeholder="150000"
              ancho="flex-1"
            />
          </span>
          <span className="flex gap-2">
            <Campo
              etiqueta="Metros mín. (m²)"
              valor={modal.borrador.metrosMin}
              alCambiar={(v) => modal.actualizar('metrosMin', v)}
              placeholder="80"
              ancho="flex-1"
            />
            <Campo
              etiqueta="Metros máx. (m²)"
              valor={modal.borrador.metrosMax}
              alCambiar={(v) => modal.actualizar('metrosMax', v)}
              placeholder="200"
              ancho="flex-1"
            />
          </span>
          <Campo
            etiqueta="Ubicación contiene"
            valor={modal.borrador.ubicacion}
            alCambiar={(v) => modal.actualizar('ubicacion', v)}
            placeholder="Villa Tocoma"
          />
          <Campo
            etiqueta="Habitaciones mín."
            valor={modal.borrador.habitacionesMin}
            alCambiar={(v) => modal.actualizar('habitacionesMin', v)}
            placeholder="3"
          />
          <span className="flex gap-2">
            <button
              type="button"
              onClick={() => {
                modal.aplicar();
                alAplicar();
              }}
              className={`flex-1 cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 ${CLASE_TEXTO}`}
            >
              Aplicar
            </button>
            <button
              type="button"
              onClick={modal.limpiar}
              className={`flex-1 cursor-pointer rounded-none border ${CLASE_BORDE} bg-transparent px-4 py-2 ${CLASE_TEXTO}`}
            >
              Limpiar
            </button>
          </span>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function Campo({
  etiqueta,
  valor,
  alCambiar,
  placeholder,
  ancho = '',
}: {
  etiqueta: string;
  valor: string;
  alCambiar: (v: string) => void;
  placeholder: string;
  ancho?: string;
}) {
  return (
    <label className={`flex flex-col gap-1 text-sm font-normal ${CLASE_TINTA} ${ancho}`}>
      {etiqueta}
      <input
        type="text"
        value={valor}
        onChange={(e) => alCambiar(e.target.value)}
        placeholder={placeholder}
        className={`w-full rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-2 text-sm font-normal outline-none placeholder:text-black/40`}
      />
    </label>
  );
}

function TipoFiltro({
  valor,
  alElegir,
}: {
  valor: TipoInmueble | null;
  alElegir: (t: TipoInmueble | null) => void;
}) {
  return (
    <label className={`flex-col gap-1 text-sm font-normal ${CLASE_TINTA} ${SOLO_MOVIL_TABLETA} flex`}>
      Tipo de inmueble
      <select
        value={valor ?? ''}
        onChange={(e) => alElegir(e.target.value === '' ? null : (e.target.value as TipoInmueble))}
        className={`w-full rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-2 text-sm font-normal outline-none`}
      >
        <option value="">Todos</option>
        {TIPOS.map((t) => (
          <option key={t} value={t}>
            {ETIQUETAS_TIPO[t]}
          </option>
        ))}
      </select>
    </label>
  );
}

function OperacionFiltro({
  valor,
  alElegir,
}: {
  valor: FiltrosAvanzados['operacion'];
  alElegir: (v: FiltrosAvanzados['operacion']) => void;
}) {
  return (
    <span className={`flex flex-col gap-1 text-sm font-normal ${CLASE_TINTA}`}>
      Operación
      <span className="flex gap-2">
        {(['todas', 'venta', 'alquiler'] as const).map((o) => (
          <button
            key={o}
            type="button"
            onClick={() => alElegir(o)}
            className={`flex-1 cursor-pointer rounded-none border ${CLASE_BORDE} px-4 py-2 ${CLASE_TEXTO} ${
              valor === o ? CLASE_ACTIVO : 'bg-transparent text-black'
            }`}
          >
            {o === 'todas' ? 'Todas' : o === 'venta' ? 'Venta' : 'Alquiler'}
          </button>
        ))}
      </span>
    </span>
  );
}
