const FORMATO_PRECIO = new Intl.NumberFormat('es-VE', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 0,
});

export function formatearPrecio(precio: number): string {
  if (!Number.isFinite(precio) || precio <= 0) return 'A convenir';
  return FORMATO_PRECIO.format(precio);
}
