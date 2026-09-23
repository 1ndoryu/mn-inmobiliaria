// Capa de persistencia abstraída para poder migrar a móvil (Capacitor/SQLite)
// o a backend REST sin tocar los componentes. Hoy: localStorage.

export interface StorageAdapter {
  leer<T>(clave: string): T | null;
  escribir<T>(clave: string, valor: T): void;
}

export class LocalStorageAdapter implements StorageAdapter {
  leer<T>(clave: string): T | null {
    try {
      const raw = localStorage.getItem(clave);
      if (!raw) return null;
      return JSON.parse(raw) as T;
    } catch {
      return null;
    }
  }

  escribir<T>(clave: string, valor: T): void {
    localStorage.setItem(clave, JSON.stringify(valor));
  }
}

export const CLAVE_INMUEBLES = 'inmobiliaria:inmuebles:v1';
export const CLAVE_BORRADOR = 'inmobiliaria:draft:v1';
