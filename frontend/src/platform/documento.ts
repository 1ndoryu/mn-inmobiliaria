/* Adaptador de plataforma (document): raíz de montaje y portapapeles.
 * Cualquier acceso directo a `document`/`navigator` vive aquí, nunca en
 * hooks/componentes. Sentinel (dom-access-outside-platform). */

/* Raíz de montaje: falla explícito si falta (nunca null silencioso). */
export function obtenerElementoPorId(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`Falta el elemento raíz #${id} en index.html.`);
  return el;
}

/* Copia texto al portapapeles; `false` = no disponible (permisos o
 * contexto no seguro). El llamador decide qué mostrar. */
export async function copiarTexto(texto: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(texto);
    return true;
  } catch {
    return false;
  }
}

/* Pide a los buscadores que no indexen la página (p. ej. piezas sueltas de
 * publicidad). Devuelve la función que restaura el `robots` previo. */
export function fijarNoIndexar(): () => void {
  const existente = document.head.querySelector<HTMLMetaElement>('meta[name="robots"]');
  const previo = existente?.getAttribute('content') ?? null;
  const meta = existente ?? document.createElement('meta');
  meta.name = 'robots';
  meta.content = 'noindex, nofollow';
  if (!existente) document.head.appendChild(meta);
  return () => {
    if (previo === null) meta.remove();
    else meta.content = previo;
  };
}

/* Cabecera propia de una página (título, meta, canónica, Open Graph,
 * Twitter y JSON-LD). Los `meta` y la canónica ya existen en `index.html`:
 * se reescriben y se restauran al desmontar. El JSON-LD se crea y se quita. */
export interface EtiquetasPagina {
  titulo: string;
  descripcion: string;
  url: string;
  imagen: string;
  datosEstructurados: Record<string, unknown>;
}

const ID_DATOS_PAGINA = 'datos-pagina';

/* Devuelve la función que deja la cabecera como estaba. */
export function fijarEtiquetasPagina(e: EtiquetasPagina): () => void {
  const tituloPrevio = document.title;
  const previos: Array<{ el: Element; nombre: string; valor: string | null }> = [];
  const poner = (el: Element, nombre: string, valor: string) => {
    previos.push({ el, nombre, valor: el.getAttribute(nombre) });
    el.setAttribute(nombre, valor);
  };
  const campos: Array<[string, string]> = [
    ['meta[name="description"]', e.descripcion],
    ['meta[property="og:title"]', e.titulo],
    ['meta[property="og:description"]', e.descripcion],
    ['meta[property="og:url"]', e.url],
    ['meta[property="og:image"]', e.imagen],
    ['meta[name="twitter:title"]', e.titulo],
    ['meta[name="twitter:description"]', e.descripcion],
    ['meta[name="twitter:image"]', e.imagen],
  ];
  document.title = e.titulo;
  for (const [selector, valor] of campos) {
    const el = document.head.querySelector(selector);
    if (el) poner(el, 'content', valor);
  }
  const canonica = document.head.querySelector('link[rel="canonical"]');
  if (canonica) poner(canonica, 'href', e.url);

  // [por que] Si el servidor ya dejó el JSON-LD de la página, se reemplaza en vez de duplicarlo.
  const existente = document.querySelector<HTMLScriptElement>(`script#${ID_DATOS_PAGINA}`);
  const script = existente ?? document.createElement('script');
  const jsonPrevio = script.textContent;
  script.type = 'application/ld+json';
  script.id = ID_DATOS_PAGINA;
  script.textContent = JSON.stringify(e.datosEstructurados);
  if (!existente) document.head.appendChild(script);

  return () => {
    document.title = tituloPrevio;
    for (const p of previos) {
      if (p.valor === null) p.el.removeAttribute(p.nombre);
      else p.el.setAttribute(p.nombre, p.valor);
    }
    if (existente) script.textContent = jsonPrevio;
    else script.remove();
  };
}
