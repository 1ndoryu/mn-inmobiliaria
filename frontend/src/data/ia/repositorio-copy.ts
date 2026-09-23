import { CLAVE_CONFIG_COPY, normalizarConfigCopy, type ConfigCopy } from '../../domain/copy';
import { LocalStorageAdapter, type StorageAdapter } from '../sesion/storage';

const storage: StorageAdapter = new LocalStorageAdapter();

/* Configuración del Copy (localStorage, valores seguros por defecto).
 * Vive aquí para que la futura app móvil la reutilice cambiando el adapter. */
export function leerConfigCopy(): ConfigCopy {
  return normalizarConfigCopy(storage.leer<unknown>(CLAVE_CONFIG_COPY));
}

export function guardarConfigCopy(config: ConfigCopy): void {
  storage.escribir(CLAVE_CONFIG_COPY, normalizarConfigCopy(config));
}
