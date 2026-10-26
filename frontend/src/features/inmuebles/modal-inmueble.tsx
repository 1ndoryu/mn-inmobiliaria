import { Sparkles } from 'lucide-react';
import {
  ESTADOS,
  ETIQUETAS_TIPO,
  TIPOS,
  draftTieneContenido,
  type Inmueble,
  type InmuebleDraft,
} from '@/domain/inmueble';
import { BannerBorrador, Etiqueta } from './formulario/campos-formulario';
import { CampoAliasTitulos } from './campos/alias-titulos';
import { CampoMarketplaceId } from './campos/marketplace-id';
import { FichaFormulario } from './formulario/ficha-formulario';
import { useFichaInmueble } from '@/hooks/inmuebles/use-ficha-inmueble';
import { FotosFormulario } from './formulario/fotos-formulario';
import { PieModalInmueble } from './formulario/pie-modal-inmueble';
import { ModalIA } from './modal-ia';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Select } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';
import { useModalInmueble } from '@/hooks/inmuebles/use-modal-inmueble';
import { confirmar } from '@/platform/ventana';

/* Borrador persistente (solo modo "nuevo"; vive en el padre para
 * sobrevivir al modal). */
interface BorradorExterno {
  borrador: InmuebleDraft;
  alCambiar: (d: InmuebleDraft) => void;
  hayGuardado: boolean;
  alRestaurar: () => void;
  alEmpezarDeCero: () => void;
  alLimpiarTrasGuardar: () => void;
}

interface Props {
  abierto: boolean;
  alCambiarAbierto: (abierto: boolean) => void;
  editando: Inmueble | null;
  borrador: BorradorExterno;
  alGuardarNuevo: (inmueble: Inmueble) => Promise<boolean>;
  alGuardarEdicion: (inmueble: Inmueble) => Promise<boolean>;
  /* Motivo del último guardado fallido: se ve aquí porque el banner de la app queda tapado por el modal. */
  errorGuardado?: string | null;
}

export function ModalInmueble(props: Props) {
  const { abierto, alCambiarAbierto } = props;
  /* Ficha /ask editable: se fusiona al guardar (pisa lo guardado). */
  const ficha = useFichaInmueble(abierto, props.editando);
  const {
    esEdicion,
    form,
    errores,
    avisoFotos,
    iaAbierto,
    setIaAbierto,
    inputFotos,
    cambiar,
    anadirFotos,
    quitarFoto,
    hacerPrincipal,
    guardar,
    guardando,
    aplicarFichaIA,
  } = useModalInmueble({
    abierto,
    editando: props.editando,
    borrador: props.borrador.borrador,
    setBorrador: props.borrador.alCambiar,
    alCambiarAbierto,
    limpiarTrasGuardar: props.borrador.alLimpiarTrasGuardar,
    onGuardarNuevo: (inm) => props.alGuardarNuevo(ficha.conFicha(inm)),
    onGuardarEdicion: (inm) => props.alGuardarEdicion(ficha.conFicha(inm)),
  });

  const mostrarBannerRecuperar =
    !esEdicion && props.borrador.hayGuardado && !draftTieneContenido(props.borrador.borrador);

  const descartar = () => {
    if (confirmar('Descartar el borrador y empezar de cero. Esta acción no se puede deshacer.')) {
      void props.borrador.alEmpezarDeCero();
    }
  };

  return (
    <>
    {/* Mientras guarda no se cierra (Esc, clic fuera o X): el cierre llega solo al terminar bien. */}
    <Dialog open={abierto} onOpenChange={(a) => (guardando && !a ? undefined : alCambiarAbierto(a))}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{esEdicion ? 'Editar inmueble' : 'Añadir inmueble'}</DialogTitle>
          <DialogDescription>
            {esEdicion
              ? 'Modifica la ficha. Cerrar sin guardar conserva la ficha original.'
              : 'Sin campos obligatorios: guarda con la información que tengas. El borrador se autoguard: puedes cerrar sin miedo, no se pierde nada.'}
          </DialogDescription>
          <Button variant="outline" className="mt-2 w-full sm:w-auto" onClick={() => setIaAbierto(true)}>
            <Sparkles className="h-4 w-4" /> Redactar con IA
          </Button>
        </DialogHeader>

        {mostrarBannerRecuperar && (
          <BannerBorrador alRestaurar={props.borrador.alRestaurar} alEmpezarDeCero={props.borrador.alEmpezarDeCero} />
        )}

        <div className="grid gap-4">
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1 sm:col-span-2">
              <Etiqueta error={errores.titulo}>Título</Etiqueta>
              <Input
                value={form.titulo}
                onChange={(e) => cambiar('titulo', e.target.value)}
                placeholder="Piso luminoso en el centro"
                aria-invalid={Boolean(errores.titulo)}
                className={cn(errores.titulo && 'border-destructive')}
              />
              {errores.titulo && <p className="text-xs text-destructive">{errores.titulo}</p>}
            </div>
            <div className="space-y-1 sm:col-span-2">
              <Etiqueta error={errores.ubicacion}>Ubicación</Etiqueta>
              <Input
                value={form.ubicacion}
                onChange={(e) => cambiar('ubicacion', e.target.value)}
                placeholder="Calle Mayor 12, Madrid"
                aria-invalid={Boolean(errores.ubicacion)}
                className={cn(errores.ubicacion && 'border-destructive')}
              />
              {errores.ubicacion && <p className="text-xs text-destructive">{errores.ubicacion}</p>}
            </div>
            <div className="space-y-1 sm:col-span-2">
              <Etiqueta>Residencia / conjunto</Etiqueta>
              <Input
                value={form.residencia}
                onChange={(e) => cambiar('residencia', e.target.value)}
                placeholder="Residencias Los Naranjos"
              />
            </div>
            {/* [18AA-6] Nombre corto: cómo se cita el inmueble en el chat del Marketplace. */}
            <div className="space-y-1 sm:col-span-2">
              <Etiqueta>Nombre corto</Etiqueta>
              <Input
                value={form.nombre_corto}
                onChange={(e) => cambiar('nombre_corto', e.target.value)}
                placeholder="Ej. Apart. en Resd. Kama Merú"
              />
              <p className="text-xs text-muted-foreground">Cómo se cita el inmueble en el chat del Marketplace.</p>
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.precio}>Precio ($)</Etiqueta>
              <Input
                value={form.precio}
                onChange={(e) => cambiar('precio', e.target.value)}
                placeholder="250000"
                inputMode="numeric"
                aria-invalid={Boolean(errores.precio)}
                className={cn(errores.precio && 'border-destructive')}
              />
              {errores.precio && <p className="text-xs text-destructive">{errores.precio}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta>Estado</Etiqueta>
              <Select
                value={form.estado}
                onChange={(e) => cambiar('estado', e.target.value as InmuebleDraft['estado'])}
              >
                {ESTADOS.map((s) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </Select>
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.tipo}>Tipo</Etiqueta>
              <Select
                value={form.tipo}
                onChange={(e) => cambiar('tipo', e.target.value as InmuebleDraft['tipo'])}
                className={errores.tipo ? 'border-destructive' : undefined}
              >
                <option value="">Seleccionar…</option>
                {TIPOS.map((t) => (
                  <option key={t} value={t}>
                    {ETIQUETAS_TIPO[t]}
                  </option>
                ))}
              </Select>
              {errores.tipo && <p className="text-xs text-destructive">{errores.tipo}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.operacion}>Operación</Etiqueta>
              <Select
                value={form.operacion}
                onChange={(e) => cambiar('operacion', e.target.value as InmuebleDraft['operacion'])}
                className={errores.operacion ? 'border-destructive' : undefined}
              >
                <option value="">Seleccionar…</option>
                <option value="venta">venta</option>
                <option value="alquiler">alquiler</option>
              </Select>
              {errores.operacion && <p className="text-xs text-destructive">{errores.operacion}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.habitaciones}>Habitaciones</Etiqueta>
                <Input
                  value={form.habitaciones}
                  onChange={(e) => cambiar('habitaciones', e.target.value)}
                  placeholder="3"
                  inputMode="numeric"
                />
              {errores.habitaciones && <p className="text-xs text-destructive">{errores.habitaciones}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.banos}>Baños</Etiqueta>
              <Input value={form.banos} onChange={(e) => cambiar('banos', e.target.value)} placeholder="2" inputMode="numeric" />
              {errores.banos && <p className="text-xs text-destructive">{errores.banos}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.metros}>M² propiedad (construidos)</Etiqueta>
              <Input
                value={form.metros}
                onChange={(e) => cambiar('metros', e.target.value)}
                placeholder="90"
                inputMode="numeric"
                className={cn(errores.metros && 'border-destructive')}
              />
              {errores.metros && <p className="text-xs text-destructive">{errores.metros}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.metrosTerreno}>M² terreno</Etiqueta>
              <Input
                value={form.metrosTerreno}
                onChange={(e) => cambiar('metrosTerreno', e.target.value)}
                placeholder="500"
                inputMode="numeric"
                className={cn(errores.metrosTerreno && 'border-destructive')}
              />
              {errores.metrosTerreno && <p className="text-xs text-destructive">{errores.metrosTerreno}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta>Puestos de estacionamiento</Etiqueta>
              <Input
                value={form.puestos}
                onChange={(e) => cambiar('puestos', e.target.value)}
                placeholder="1"
                inputMode="numeric"
              />
            </div>
            <CampoMarketplaceId
              valor={form.marketplaceId}
              error={errores.marketplaceId}
              alCambiar={(v) => cambiar('marketplaceId', v)}
            />
            <CampoAliasTitulos
              valor={form.aliasTitulos}
              error={errores.aliasTitulos}
              alCambiar={(v) => cambiar('aliasTitulos', v)}
            />
            <div className="space-y-1 sm:col-span-2">
              <Etiqueta>Descripción</Etiqueta>
              <Textarea
                value={form.descripcion}
                onChange={(e) => cambiar('descripcion', e.target.value)}
                placeholder="Orientación, plantas, extras, estado de conservación…"
              />
            </div>
            <div className="sm:col-span-2">
              <FichaFormulario tipo={form.tipo} extras={ficha.extras} alCambiarExtra={ficha.cambiarExtra} />
            </div>
            <FotosFormulario
              fotos={form.fotos}
              aviso={avisoFotos}
              alElegir={() => inputFotos.current?.click()}
              alQuitar={quitarFoto}
              alHacerPrincipal={hacerPrincipal}
              inputRef={inputFotos}
              alSeleccionar={(files) => void anadirFotos(files)}
            />
          </div>
        </div>

        <PieModalInmueble
          esEdicion={esEdicion}
          guardando={guardando}
          puedeDescartar={!esEdicion && draftTieneContenido(form)}
          errorGuardado={props.errorGuardado}
          alDescartar={descartar}
          alCerrar={() => alCambiarAbierto(false)}
          alGuardar={() => void guardar()}
        />
      </DialogContent>
    </Dialog>
    <ModalIA
      abierto={iaAbierto}
      alCambiarAbierto={setIaAbierto}
      textoInicial={[form.titulo, form.descripcion].filter(Boolean).join('\n')}
      fotosIniciales={form.fotos}
      onAplicar={aplicarFichaIA}
    />
    </>
  );
}
