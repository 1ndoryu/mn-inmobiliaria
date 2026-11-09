/* [30AA-1] Bloque de versionado del modal de inmueble: une el estado
 * (`useVersionado`) con las pestañas (`PanelVersiones`). Vive aquí y no en el
 * modal para que ese archivo no crezca con una sección que solo aplica al
 * editar un inmueble ya guardado. */
import { leerConfig } from '@/data/mejora/repositorio-fotos';
import { paresDeFotos, useVersionado } from '@/hooks/inmuebles/use-versionado';
import { urlDescargaDe } from '@/hooks/inmuebles/use-modal-descargar-fotos';
import type { Inmueble } from '@/domain/inmueble';
import { PanelVersiones } from './panel-versiones';

export function SeccionVersionado({ inmueble }: { inmueble: Inmueble }) {
  /* Las fotos de la variante son las mismas que exporta el escritorio: la
   * mejorada del servidor cuando existe, si no el original. */
  const fotosParaVariante = inmueble.fotos.map((_, i) => urlDescargaDe(inmueble, i));
  const versionado = useVersionado({
    inmuebleId: inmueble.id,
    fotosParaVariante,
    promptVariante: leerConfig().promptVariante,
  });

  return (
    <PanelVersiones
      datos={versionado.datos}
      cuentas={versionado.cuentas}
      vinculos={versionado.vinculos}
      ocupado={versionado.ocupado}
      aviso={versionado.aviso}
      error={versionado.error}
      progreso={versionado.progreso}
      pares={paresDeFotos(fotosParaVariante, versionado.datos?.fotos ?? [])}
      alGenerarTexto={(v, i) => void versionado.generar(v, i)}
      alGuardarTexto={(v, t, d, e) => void versionado.guardarTexto(v, t, d, e)}
      alBorrarTexto={(v) => void versionado.borrarTexto(v)}
      alGenerarFotos={() => void versionado.generarFotos()}
      alBorrarFoto={(orden) => void versionado.borrarFoto(orden)}
      alVincular={(c, m) => void versionado.vincular(c, m)}
      alDesvincular={(c) => void versionado.desvincular(c)}
      alCrearCuenta={(n, u) => void versionado.crear(n, u)}
    />
  );
}
