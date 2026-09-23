import { ImagePlus, X } from 'lucide-react';
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { urlFotoSolicitud } from '../../../data/publicar/api';
import { MAX_FOTOS_SOLICITUD, type BorradorSolicitud } from '../../../domain/solicitud';
import type { useModalPublicar } from '../../../hooks/publica/modales/use-modal-publicar';
import {
  CLASE_ACTIVO,
  CLASE_BORDE,
  CLASE_FONDO,
  CLASE_RELLENO_SUAVE,
  CLASE_TEXTO,
  CLASE_TINTA,
} from '../disenno';

/* [169A-2] Formulario "Publicar mi inmueble": cuadrado, tinta, sin sombras,
 * Söhne 400 sin negritas. Cierra con overlay o Escape (sin X); el borrador
 * no se pierde al cerrar. Las fotos se suben al elegirlas. */
export function ModalPublicar({ modal }: { modal: ReturnType<typeof useModalPublicar> }) {
  const { abierto, cerrar, borrador, actualizar, agregarFotos, quitarFoto, enviar, envio } = modal;
  const ocupada = envio.fase === 'subiendo' || envio.fase === 'enviando';

  return (
    <Dialog open={abierto} onOpenChange={(a) => !a && cerrar()}>
      <DialogContent
        showCloseButton={false}
        className={`max-h-[92dvh] overflow-y-auto rounded-none border ${CLASE_BORDE} ${CLASE_FONDO} ${CLASE_TINTA} p-6 font-soehne font-normal sm:max-w-2xl`}
      >
        <DialogTitle className="font-soehne text-xl leading-tight font-normal">
          Publicar mi inmueble
        </DialogTitle>
        {envio.fase === 'enviada' ? (
          <Exito alCerrar={cerrar} />
        ) : (
          <form
            className="mt-4 flex flex-col gap-4 font-normal"
            onSubmit={(e) => {
              e.preventDefault();
              void enviar();
            }}
          >
            <Campo
              etiqueta="Nombre"
              valor={borrador.nombre}
              alCambiar={(v) => actualizar('nombre', v)}
              placeholder="Tu nombre"
            />
            <div className="grid gap-4 sm:grid-cols-2">
              <Campo
                etiqueta="Teléfono"
                valor={borrador.telefono}
                alCambiar={(v) => actualizar('telefono', v)}
                placeholder="+58 424 9208855"
                tipo="tel"
              />
              <Campo
                etiqueta="Correo (opcional)"
                valor={borrador.email}
                alCambiar={(v) => actualizar('email', v)}
                placeholder="tucorreo@ejemplo.com"
                tipo="email"
              />
            </div>
            <Campo
              etiqueta="Ubicación"
              valor={borrador.ubicacion}
              alCambiar={(v) => actualizar('ubicacion', v)}
              placeholder="Calle, barrio, municipio…"
            />
            <div className="grid gap-4 sm:grid-cols-2">
              <Campo
                etiqueta="Residencia / conjunto (opcional)"
                valor={borrador.residencia}
                alCambiar={(v) => actualizar('residencia', v)}
                placeholder="Residencias Los Naranjos"
              />
              <Campo
                etiqueta="Puestos de estacionamiento"
                valor={borrador.puestos}
                alCambiar={(v) => actualizar('puestos', v)}
                placeholder="1"
                tipo="number"
              />
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <Campo
                etiqueta="Precio estimado ($, opcional)"
                valor={borrador.precioEstimado}
                alCambiar={(v) => actualizar('precioEstimado', v)}
                placeholder="150000"
                tipo="text"
              />
              <Operacion valor={borrador.operacion} alElegir={(v) => actualizar('operacion', v)} />
            </div>
            <label className={`flex flex-col gap-1 text-sm font-normal ${CLASE_TINTA}`}>
              Descripción
              <textarea
                value={borrador.descripcion}
                onChange={(e) => actualizar('descripcion', e.target.value)}
                rows={4}
                placeholder="Habitaciones, metros, estado, lo que la hace especial…"
                className={`w-full resize-y rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-2 text-sm font-normal outline-none placeholder:text-black/40`}
              />
            </label>
            <Fotos
              claves={borrador.fotosClaves}
              ocupada={ocupada}
              alElegir={(f) => void agregarFotos(f)}
              alQuitar={quitarFoto}
            />
            {envio.error && (
              <p role="alert" className="text-sm font-normal text-red-800">
                {envio.error}
              </p>
            )}
            <button
              type="submit"
              disabled={ocupada}
              className={`cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 ${CLASE_TEXTO} disabled:cursor-wait disabled:opacity-60`}
            >
              {envio.fase === 'enviando'
                ? 'Enviando…'
                : envio.fase === 'subiendo'
                  ? 'Subiendo fotos…'
                  : 'Enviar solicitud'}
            </button>
          </form>
        )}
      </DialogContent>
    </Dialog>
  );
}

function Exito({ alCerrar }: { alCerrar: () => void }) {
  return (
    <div className="mt-4 flex flex-col gap-4 font-normal">
      <p className={`text-sm leading-relaxed font-normal ${CLASE_TINTA}`}>
        Solicitud enviada. La revisaremos y te contactaremos en breve.
      </p>
      <button
        type="button"
        onClick={alCerrar}
        className={`cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 ${CLASE_TEXTO}`}
      >
        Cerrar
      </button>
    </div>
  );
}

function Campo({
  etiqueta,
  valor,
  alCambiar,
  placeholder,
  tipo = 'text',
}: {
  etiqueta: string;
  valor: string;
  alCambiar: (v: string) => void;
  placeholder: string;
  tipo?: string;
}) {
  return (
    <label className={`flex flex-col gap-1 text-sm font-normal ${CLASE_TINTA}`}>
      {etiqueta}
      <input
        type={tipo}
        value={valor}
        onChange={(e) => alCambiar(e.target.value)}
        placeholder={placeholder}
        className={`w-full rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-2 text-sm font-normal outline-none placeholder:text-black/40`}
      />
    </label>
  );
}

function Operacion({
  valor,
  alElegir,
}: {
  valor: BorradorSolicitud['operacion'];
  alElegir: (v: BorradorSolicitud['operacion']) => void;
}) {
  return (
    <span className={`flex flex-col gap-1 text-sm font-normal ${CLASE_TINTA}`}>
      Operación
      <span className="flex gap-2">
        {(['venta', 'alquiler'] as const).map((o) => (
          <button
            key={o}
            type="button"
            onClick={() => alElegir(o)}
            className={`flex-1 cursor-pointer rounded-none border ${CLASE_BORDE} px-4 py-2 ${CLASE_TEXTO} ${
              valor === o ? CLASE_ACTIVO : 'bg-transparent text-black'
            }`}
          >
            {o === 'venta' ? 'Venta' : 'Alquiler'}
          </button>
        ))}
      </span>
    </span>
  );
}

function Fotos({
  claves,
  ocupada,
  alElegir,
  alQuitar,
}: {
  claves: string[];
  ocupada: boolean;
  alElegir: (f: FileList) => void;
  alQuitar: (clave: string) => void;
}) {
  return (
    <span className={`flex flex-col gap-2 text-sm font-normal ${CLASE_TINTA}`}>
      Fotos ({claves.length}/{MAX_FOTOS_SOLICITUD})
      {claves.length > 0 && (
        <span className="flex flex-wrap gap-2">
          {claves.map((c) => (
            <span key={c} className={`relative border ${CLASE_BORDE} ${CLASE_RELLENO_SUAVE}`}>
              <img src={urlFotoSolicitud(`/uploads/${c}`)} alt="" className="h-16 w-16 rounded-none object-cover" />
              <button
                type="button"
                aria-label="Quitar foto"
                onClick={() => alQuitar(c)}
                className="absolute top-0 right-0 cursor-pointer rounded-none bg-black/60 p-0.5"
              >
                <X className="h-3 w-3 text-white" />
              </button>
            </span>
          ))}
        </span>
      )}
      {claves.length < MAX_FOTOS_SOLICITUD && (
        <label
          className={`flex cursor-pointer items-center gap-2 rounded-none border ${CLASE_BORDE} bg-transparent px-4 py-2 ${CLASE_TEXTO} ${
            ocupada ? 'pointer-events-none opacity-60' : ''
          }`}
        >
          <ImagePlus className="h-4 w-4" />
          {ocupada ? 'Subiendo…' : 'Añadir fotos (jpg, png, webp)'}
          <input
            type="file"
            accept="image/jpeg,image/png,image/webp"
            multiple
            className="hidden"
            disabled={ocupada}
            onChange={(e) => {
              if (e.target.files) alElegir(e.target.files);
              e.target.value = '';
            }}
          />
        </label>
      )}
    </span>
  );
}
