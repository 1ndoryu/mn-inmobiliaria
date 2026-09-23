import type { InmuebleDraft } from '@/domain/inmueble';
import type { FichaIA } from '@/data/ia/ia';

/* Fusión pura de la ficha IA en el formulario: solo rellena los campos que
 * la IA pudo determinar; las fotos nuevas se añaden sin duplicar.
 * Extraído de `modal-inmueble` (Sentinel limite-lineas). */
export function aplicarFichaAlDraft(form: InmuebleDraft, ficha: FichaIA, fotosIA: string[]): InmuebleDraft {
  const next: InmuebleDraft = { ...form };
  if (ficha.titulo) next.titulo = ficha.titulo;
  if (ficha.descripcion) next.descripcion = ficha.descripcion;
  if (ficha.ubicacion) next.ubicacion = ficha.ubicacion;
  if (ficha.residencia) next.residencia = ficha.residencia;
  if (ficha.precio !== null) next.precio = String(ficha.precio);
  if (ficha.tipo) next.tipo = ficha.tipo;
  if (ficha.operacion) next.operacion = ficha.operacion;
  if (ficha.habitaciones !== null) next.habitaciones = String(ficha.habitaciones);
  if (ficha.banos !== null) next.banos = String(ficha.banos);
  if (ficha.metros !== null) next.metros = String(ficha.metros);
  if (ficha.metrosTerreno !== null) next.metrosTerreno = String(ficha.metrosTerreno);
  if (ficha.puestos !== null) next.puestos = String(ficha.puestos);
  const nuevas = fotosIA.filter((f) => !next.fotos.includes(f));
  if (nuevas.length > 0) next.fotos = [...next.fotos, ...nuevas];
  return next;
}
