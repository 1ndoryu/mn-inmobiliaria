#!/usr/bin/env bash
# [GW-14] Copia diaria de la BD de MN-Inmobiliaria en la 89.
#
# Por qué aquí y no en /var/tmp ni en ~/.cache: `cm-sweep` los barre sin
# umbral y se llevaría la copia a medias (AGENTS §1). `~/.local/share` está en
# la lista NUNCA del barrido.
#
# Por qué no lo hace ya producción: prod tiene sus copias; esto cubre la 89,
# que desde 2026-10-14 es la fuente real (26 inmuebles frente a 20 del portátil).
#
# Uso: scripts/dev/respaldo-bd.sh [--retencion 14] [--bd glory_backend_inmobiliaria]
# Cron: 20 3 * * * /usr/bin/bash $HOME/taller/MN-Inmobiliaria/scripts/dev/respaldo-bd.sh >/dev/null 2>&1
set -euo pipefail

BD=glory_backend_inmobiliaria
RETENCION=14
while [ $# -gt 0 ]; do
  case "$1" in
    --bd) BD="$2"; shift 2 ;;
    --retencion) RETENCION="$2"; shift 2 ;;
    *) echo "uso: $0 [--retencion N] [--bd nombre]" >&2; exit 2 ;;
  esac
done

DESTINO="$HOME/.local/share/MN-Inmobiliaria/respaldos"
mkdir -p "$DESTINO"
SELLO="$(date +%Y%m%d-%H%M)"
TMP="$(mktemp "$DESTINO/.parcial-XXXXXX")"
FINAL="$DESTINO/$BD-$SELLO.dump"

# Falla si el dump está vacío o truncado: una copia que no restaura es peor que
# ninguna, porque da por cubierto lo que no lo está.
pg_dump -h /var/run/postgresql -Fc -d "$BD" -f "$TMP"
if ! pg_restore --list "$TMP" >/dev/null; then
  rm -f "$TMP"
  echo "FALLO: el dump no es válido (pg_restore --list)" >&2
  exit 1
fi
mv "$TMP" "$FINAL"

# Retención: se cuentan solo las copias completas de esta BD.
mapfile -t viejas < <(ls -1t "$DESTINO/$BD"-*.dump 2>/dev/null | tail -n "+$((RETENCION + 1))")
for f in "${viejas[@]:-}"; do
  [ -n "$f" ] && rm -f "$f"
done

echo "$(du -h "$FINAL" | cut -f1) $FINAL (retención: $RETENCION)"
