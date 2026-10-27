import { ETIQUETAS_TIPO, portadaDe, type InmueblePublico } from '../../../domain/inmueble';
import type { EtiquetasPagina } from '../../../platform/documento';
import { rutaInmueble } from './ruta-inmueble';

const IMAGEN_POR_DEFECTO = '/img/presentacion.jpg';
const LARGO_DESCRIPCION = 160;

/* Cabecera SEO de un inmueble (título, descripción, Open Graph y JSON-LD).
 * `origen` es la base absoluta del sitio (`origenActual()` en el navegador). */
export function datosSeoInmueble(i: InmueblePublico, origen: string): EtiquetasPagina {
  const url = `${origen}${rutaInmueble(i.slug)}`;
  const descripcion = textoDescripcion(i);
  const imagen = new URL(portadaDe(i) || IMAGEN_POR_DEFECTO, origen).href;
  return {
    titulo: `${i.titulo} - MN Inmobiliaria`,
    descripcion,
    url,
    imagen,
    datosEstructurados: {
      '@context': 'https://schema.org',
      '@type': 'RealEstateListing',
      name: i.titulo,
      description: descripcion,
      url,
      image: imagen,
      ...(i.precio > 0 ? { offers: { '@type': 'Offer', price: i.precio, priceCurrency: 'USD', url } } : {}),
    },
  };
}

function textoDescripcion(i: InmueblePublico): string {
  const operacion = i.operacion === 'ambas' ? 'venta y alquiler' : i.operacion === 'venta' ? 'venta' : 'alquiler';
  const generado = `${ETIQUETAS_TIPO[i.tipo]} en ${operacion} en ${i.ubicacion.trim() || 'Puerto Ordaz'}. Contacto directo por WhatsApp.`;
  const texto = i.descripcion.trim() || generado;
  return texto.length > LARGO_DESCRIPCION ? `${texto.slice(0, LARGO_DESCRIPCION - 1).trimEnd()}…` : texto;
}
