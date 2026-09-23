// Persistencia de fotos en IndexedDB: cabe original + mejorada en full-res
// sin reventar el límite ~5MB de localStorage. Solo vive en `data/`; los
// componentes acceden vía repositorio y hooks, nunca a indexedDB directo.
//
// Dos tablas:
// - `fotos-mejora`: copias de mejora (original intacto + mejorada).
// - `fotos-inmueble`: fotos del inmueble por id ({inmuebleId, fotos});
//   la clave 'borrador' guarda las fotos del borrador de nuevo inmueble.

import type { FotoMejora } from '../../domain/foto-mejora';

const BD = 'inmobiliaria';
const TABLA_MEJORA = 'fotos-mejora';
const TABLA_FOTOS = 'fotos-inmueble';
const VERSION = 2;

function abrirBd(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === 'undefined') {
      reject(new Error('Este navegador no permite guardar fotos (sin IndexedDB).'));
      return;
    }
    const req = indexedDB.open(BD, VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(TABLA_MEJORA)) {
        const store = db.createObjectStore(TABLA_MEJORA, { keyPath: 'id' });
        store.createIndex('por-inmueble', 'inmuebleId', { unique: false });
      }
      if (!db.objectStoreNames.contains(TABLA_FOTOS)) {
        db.createObjectStore(TABLA_FOTOS, { keyPath: 'inmuebleId' });
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error('No se pudo abrir el almacén de fotos.'));
  });
}

function tx<T>(
  db: IDBDatabase,
  tabla: string,
  modo: IDBTransactionMode,
  fn: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  return new Promise((resolve, reject) => {
    const trx = db.transaction(tabla, modo);
    const store = trx.objectStore(tabla);
    let valor: T;
    try {
      const req = fn(store);
      req.onsuccess = () => {
        valor = req.result;
      };
      req.onerror = () => reject(req.error ?? new Error('Error en el almacén de fotos.'));
    } catch (e) {
      reject(e instanceof Error ? e : new Error('Error en el almacén de fotos.'));
      return;
    }
    trx.oncomplete = () => resolve(valor);
    trx.onerror = () => reject(trx.error ?? new Error('No se pudo guardar la foto.'));
  });
}

/* Guarda o reemplaza una foto por id. */
export async function idbGuardar(foto: FotoMejora): Promise<void> {
  const db = await abrirBd();
  try {
    await tx(db, TABLA_MEJORA, 'readwrite', (store) => store.put(foto));
  } finally {
    db.close();
  }
}

export async function idbListar(): Promise<FotoMejora[]> {
  const db = await abrirBd();
  try {
    return await tx(db, TABLA_MEJORA, 'readonly', (store) => store.getAll());
  } finally {
    db.close();
  }
}

/* Borra una entrada de mejora por id (purga de duplicadas). */
export async function idbBorrarMejora(id: string): Promise<void> {
  const db = await abrirBd();
  try {
    await tx(db, TABLA_MEJORA, 'readwrite', (store) => store.delete(id));
  } finally {
    db.close();
  }
}

export async function idbBorrarPorInmueble(inmuebleId: string): Promise<void> {  const db = await abrirBd();
  try {
    const todas = await tx<FotoMejora[]>(db, TABLA_MEJORA, 'readonly', (store) => store.getAll());
    const trx = db.transaction(TABLA_MEJORA, 'readwrite');
    const store = trx.objectStore(TABLA_MEJORA);
    for (const f of todas) {
      if (f.inmuebleId === inmuebleId) store.delete(f.id);
    }
    await new Promise<void>((resolve, reject) => {
      trx.oncomplete = () => resolve();
      trx.onerror = () => reject(trx.error ?? new Error('No se pudo borrar las fotos.'));
    });
  } finally {
    db.close();
  }
}

interface RegistroFotos {
  inmuebleId: string;
  fotos: string[];
}

/* Guarda o reemplaza las fotos de un inmueble (o del borrador con clave 'borrador').
 * Se escribe el array completo: quien guarda es dueño de la foto final. */
export async function idbGuardarFotos(inmuebleId: string, fotos: string[]): Promise<void> {
  const db = await abrirBd();
  try {
    const registro: RegistroFotos = { inmuebleId, fotos: [...fotos] };
    await tx(db, TABLA_FOTOS, 'readwrite', (store) => store.put(registro));
  } finally {
    db.close();
  }
}

/* Lee las fotos; null si no hay registro (distinto de error: el llamador decide). */
export async function idbLeerFotos(inmuebleId: string): Promise<string[] | null> {
  const db = await abrirBd();
  try {
    const registro = await tx<RegistroFotos | undefined>(db, TABLA_FOTOS, 'readonly', (store) =>
      store.get(inmuebleId),
    );
    return registro ? [...registro.fotos] : null;
  } finally {
    db.close();
  }
}

/* Lee todas las fotos de inmuebles de una vez (arranque). */
export async function idbLeerTodasFotos(): Promise<Record<string, string[]>> {
  const db = await abrirBd();
  try {
    const registros = await tx<RegistroFotos[]>(db, TABLA_FOTOS, 'readonly', (store) => store.getAll());
    const mapa: Record<string, string[]> = {};
    for (const r of registros) mapa[r.inmuebleId] = [...r.fotos];
    return mapa;
  } finally {
    db.close();
  }
}

export async function idbBorrarFotos(inmuebleId: string): Promise<void> {
  const db = await abrirBd();
  try {
    await tx(db, TABLA_FOTOS, 'readwrite', (store) => store.delete(inmuebleId));
  } finally {
    db.close();
  }
}
