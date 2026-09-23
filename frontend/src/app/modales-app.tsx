import type { ConfigCopy } from '@/domain/copy';
import type { ConfigMejora, FotoMejora } from '@/domain/foto-mejora';
import type { Inmueble } from '@/domain/inmueble';
import type { useAnadirFotos } from '@/hooks/inmuebles/use-anadir-fotos';
import type { useBorrador } from '@/hooks/inmuebles/use-borrador';
import type { useTema } from '@/hooks/app/use-tema';
import { ModalAnadirFotos } from '@/features/inmuebles/modal-anadir-fotos';
import { ModalConfig } from '@/features/configuracion/modal-config';
import { ModalCopy } from '@/features/inmuebles/modal-copy';
import { ModalInmueble } from '@/features/inmuebles/modal-inmueble';
import { ModalVerInmueble } from '@/features/inmuebles/modal-ver-inmueble';

interface GrupoVista {
  viendo: Inmueble | null;
  alCerrar: () => void;
  alPublicar: (id: string, publicado: boolean) => void;
  alEditar: (inmueble: Inmueble) => void;
}

interface GrupoAjustes {
  abierta: boolean;
  alCambiar: (abierta: boolean) => void;
  tema: ReturnType<typeof useTema>;
  mejora: ConfigMejora;
  alCambiarMejora: (config: ConfigMejora) => void;
  copia: ConfigCopy;
  alCambiarCopia: (config: ConfigCopy) => void;
}

interface GrupoCopia {
  inmueble: Inmueble | null;
  generando: boolean;
  error: string | null;
  alGenerar: () => void;
  alCerrar: () => void;
}

interface GrupoEdicion {
  nuevoAbierto: boolean;
  alCambiarNuevo: (abierto: boolean) => void;
  editando: Inmueble | null;
  alCerrarEdicion: () => void;
  borrador: ReturnType<typeof useBorrador>;
  alCrear: (inmueble: Inmueble) => Promise<boolean>;
  alActualizar: (inmueble: Inmueble) => Promise<boolean>;
}

interface ModalesAppProps {
  nuevos: ReturnType<typeof useAnadirFotos>;
  inmuebles: Inmueble[];
  fotos: FotoMejora[];
  vista: GrupoVista;
  ajustes: GrupoAjustes;
  copia: GrupoCopia;
  edicion: GrupoEdicion;
}

/* Los seis modales del panel en un solo punto de composición. */
export function ModalesApp({ nuevos, inmuebles, fotos, vista, ajustes, copia, edicion }: ModalesAppProps) {
  const b = edicion.borrador;

  return (
    <>
      <ModalAnadirFotos
        inmueble={nuevos.anadiendoId ? (inmuebles.find((i) => i.id === nuevos.anadiendoId) ?? null) : null}
        alCambiarAbierto={(abierto) => {
          if (!abierto) nuevos.setAnadiendoId(null);
        }}
        onConfirmar={(nuevas) => void nuevos.confirmar(nuevas)}
      />

      <ModalVerInmueble
        inmueble={vista.viendo}
        fotosMejora={fotos}
        alCambiarAbierto={(abierto) => {
          if (!abierto) vista.alCerrar();
        }}
        onPublicar={vista.alPublicar}
        onEditar={vista.alEditar}
      />

      <ModalConfig
        abierto={ajustes.abierta}
        alCambiarAbierto={ajustes.alCambiar}
        tema={ajustes.tema.tema}
        alCambiarTema={ajustes.tema.setTema}
        configMejora={ajustes.mejora}
        alCambiarMejora={ajustes.alCambiarMejora}
        configCopy={ajustes.copia}
        alCambiarCopy={ajustes.alCambiarCopia}
      />

      <ModalCopy
        inmueble={copia.inmueble}
        generando={copia.generando}
        error={copia.error}
        onGenerar={copia.alGenerar}
        alCambiarAbierto={(abierto) => {
          if (!abierto) copia.alCerrar();
        }}
      />

      <ModalInmueble
        abierto={edicion.nuevoAbierto}
        alCambiarAbierto={edicion.alCambiarNuevo}
        editando={null}
        borrador={{
          borrador: b.borrador,
          alCambiar: b.setBorrador,
          hayGuardado: b.hayBorradorGuardado,
          alRestaurar: b.restaurarBorrador,
          alEmpezarDeCero: b.empezarDeCero,
          alLimpiarTrasGuardar: b.limpiarTrasGuardar,
        }}
        alGuardarNuevo={edicion.alCrear}
        alGuardarEdicion={edicion.alActualizar}
      />

      <ModalInmueble
        abierto={edicion.editando !== null}
        alCambiarAbierto={(abierto) => {
          if (!abierto) edicion.alCerrarEdicion();
        }}
        editando={edicion.editando}
        borrador={{
          borrador: b.borrador,
          alCambiar: b.setBorrador,
          hayGuardado: false,
          alRestaurar: b.restaurarBorrador,
          alEmpezarDeCero: b.empezarDeCero,
          alLimpiarTrasGuardar: b.limpiarTrasGuardar,
        }}
        alGuardarNuevo={edicion.alCrear}
        alGuardarEdicion={edicion.alActualizar}
      />
    </>
  );
}
