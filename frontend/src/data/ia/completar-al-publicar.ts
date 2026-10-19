import { draftAInmueble, inmuebleADraft, type Inmueble, type InmuebleDraft } from '@/domain/inmueble';
import { fichaResumen } from '@/data/ia/ficha-texto';
import { organizarConIA } from '@/data/ia/ia';
import { aplicarFichaAlDraft } from './aplicar-ficha-ia';

/* [18AA-7] Publicar no debería dejar huecos: antes de hacerlo visible, la IA
 * recibe la ficha tal como está y escribe lo que falte. Solo rellena campos
 * vacíos (`aplicarFichaAlDraft`), así que nunca pisa lo escrito a mano.
 *
 * `metrosTerreno` y `puestos` no cuentan como hueco: faltan de verdad en
 * muchos inmuebles (un apartamento no tiene parcela) y no vale gastar una
 * llamada de IA por ellos. Si el modelo los devuelve, se aprovechan igual. */

const CAMPOS_IA: readonly (keyof InmuebleDraft)[] = [
  'titulo',
  'descripcion',
  'ubicacion',
  'residencia',
  'nombre_corto',
  'precio',
  'tipo',
  'operacion',
  'habitaciones',
  'banos',
  'metros',
  'metrosTerreno',
  'puestos',
];

function faltaAlgo(i: Inmueble): boolean {
  const vacio = (v: string) => v.trim() === '';
  return (
    vacio(i.titulo) ||
    vacio(i.descripcion) ||
    vacio(i.ubicacion) ||
    vacio(i.residencia) ||
    vacio(i.nombreCorto) ||
    i.precio <= 0 ||
    i.habitaciones <= 0 ||
    i.banos <= 0 ||
    i.metros <= 0
  );
}

/** El inmueble con lo que la IA pudo completar, o `null` si no había nada que
 * completar, si la IA no respondió o si no cambió nada (así no se gasta un PUT
 * —ni se toca `updated_at`— para nada). Quien llama publica igual. */
export async function completarAlPublicar(i: Inmueble, signal?: AbortSignal): Promise<Inmueble | null> {
  if (!faltaAlgo(i)) return null;
  const r = await organizarConIA({ texto: fichaResumen(i), fotos: [], signal });
  if (!r.ok) return null;
  const antes = inmuebleADraft(i);
  const despues = aplicarFichaAlDraft(antes, r.ficha, []);
  if (CAMPOS_IA.every((c) => antes[c] === despues[c])) return null;
  return draftAInmueble(despues, i);
}
