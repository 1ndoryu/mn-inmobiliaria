/* Adaptador de plataforma (window): temporizadores y aviso de cierre.
 * Todo acceso directo a `window` vive aquí, nunca en hooks/componentes.
 * Sentinel (window-reference-outside-platform). */

/* Ejecuta `fn` tras `ms`; devuelve función para cancelarlo (debounce). */
export function temporizar(ms: number, fn: () => void): () => void {
  const t = window.setTimeout(fn, ms);
  return () => window.clearTimeout(t);
}

/* Aviso nativo al recargar/cerrar la pestaña mientras `debeAvisar()` sea
 * verdad; devuelve función para retirar el aviso. */
export function avisarAlCerrar(debeAvisar: () => boolean): () => void {
  const handler = (e: BeforeUnloadEvent) => {
    if (debeAvisar()) e.preventDefault();
  };
  window.addEventListener('beforeunload', handler);
  return () => window.removeEventListener('beforeunload', handler);
}

/* Ruta actual sin query (decide pública vs admin). */
export function rutaActual(): string {
  return window.location.pathname;
}

/* Navegación completa a `ruta` (pública ↔ admin son árboles distintos). */
export function irA(ruta: string): void {
  window.location.assign(ruta);
}

/* Confirmación nativa (descartes y reinicios destructivos). */
export function confirmar(mensaje: string): boolean {
  return window.confirm(mensaje);
}

/* Suscribe `fn` a un evento de ventana; devuelve función para soltarlo. */
export function suscribirEvento(nombre: string, fn: () => void): () => void {
  window.addEventListener(nombre, fn);
  return () => window.removeEventListener(nombre, fn);
}
