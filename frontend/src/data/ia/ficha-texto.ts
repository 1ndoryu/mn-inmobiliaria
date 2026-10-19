import { ETIQUETAS_TIPO, formatearPrecio, type Inmueble } from '@/domain/inmueble';

/* La ficha del inmueble como texto para la IA. La usan el copy de redes
 * (`copy-ia`) y el completado al publicar (`completar-ficha`): el modelo
 * recibe siempre los mismos datos, en el mismo orden y con las mismas
 * etiquetas, y así no hay dos resúmenes que se desvíen. */
export function fichaResumen(i: Inmueble): string {
  const partes = [
    `Título: ${i.titulo || 'Sin título'}`,
    `Descripción: ${i.descripcion || '—'}`,
    `Ubicación: ${i.ubicacion || '—'}`,
    i.residencia ? `Residencia: ${i.residencia}` : 'Residencia: —',
    i.precio > 0 ? `Precio: ${formatearPrecio(i.precio)}` : 'Precio: a consultar',
    `Tipo: ${ETIQUETAS_TIPO[i.tipo]} · Operación: ${i.operacion} · Estado: ${i.estado}`,
  ];
  const extras: string[] = [];
  if (i.habitaciones > 0) extras.push(`${i.habitaciones} hab`);
  if (i.banos > 0) extras.push(`${i.banos} baños`);
  if (i.metros > 0) extras.push(`${i.metros} m²`);
  if (i.metrosTerreno > 0) extras.push(`${i.metrosTerreno} m² terreno`);
  if (i.puestos > 0) extras.push(`${i.puestos} puestos`);
  if (extras.length > 0) partes.push(`Detalles: ${extras.join(', ')}`);
  return partes.join('\n');
}
