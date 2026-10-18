/* [09AA-21] Matriz de `claves_cache`: ID exacto válido (ficha + conocido),
 * ID exacto inexistente (sin ficha, sin fallback al título), sin ID con
 * título que empareja (ficha por título) y UUID legacy intacto (ficha por
 * UUID). Humo contra la BD real de rama (`DATABASE_URL`); sin ella se omite.
 * El borrador jamás se bloquea: los casos negativos dan `SIN_FICHA`. */
mod claves_cache_mp_id;
/* [10AA-17] Regenerar (`force`): filtro de hit y persistencia con/sin pisar. */
mod regenerar_force;
