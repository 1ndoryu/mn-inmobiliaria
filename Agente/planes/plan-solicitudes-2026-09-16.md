# Plan 169A-2 — Solicitudes "Publicar mi inmueble" (2026-09-16)

## Objetivo
Modal público para que clientes propongan inmuebles (pendientes de revisión).
Front `INMOBILIARIA` + backend `MN-Inmobiliaria` (rama `inmobiliaria`).

## Alcance
- Backend: tabla `solicitudes`, subida pública de fotos (reutiliza
  validación magic-bytes + 10 MiB), `POST /api/public/solicitudes`,
  `POST /api/public/solicitudes/fotos`, `GET /api/admin/solicitudes`
  y `PATCH /api/admin/solicitudes/:id` (JWT). Validación formato
  básico de email (validator) y teléfono 6–15 dígitos.
- Front: `ModalPublicar` cuadrado (tokens `disenno.ts`), borrador
  persistente al cerrar (localStorage; las fotos ya subidas son claves,
  no pesan), fotos vía endpoint público. Sin UI admin de revisión
  (queda pendiente explícito).

## Fases
1. [x] Decisiones de diseño (fotos=upload, alcance=recepción+revisión, validación=básica)
2. [ ] Backend: migración `20260916000006_solicitudes`
3. [ ] Backend: `models/solicitud.rs` + `repositories/solicitud.rs` (+mods)
4. [ ] Backend: refactor `guardar_archivo_foto` reutilizable en servicio
5. [ ] Backend: `handlers/solicitudes.rs` + cableado en `mod.rs` (¡hay cambios sin commitear de 169A-1/chat: solo añadir líneas!)
6. [ ] Backend: migrar local + `cargo fmt/check/clippy/test` (target en `C:\tmp`)
7. [ ] Front: tipos + API `data/publicar/` + hook `use-modal-publicar` + `ModalPublicar` + cablear `CabeceraPublica`
8. [ ] Verificación: `tsc`, alta real contra backend local, commit por repo
9. [ ] Roadmap + completadas + lecciones

## Estado (16/09 13:20 UTC)
Fases 1-7 implementadas. Backend convergido con 169A-4/7 y verificado E2E
en :3001. Front modal creado (`tsc` 0, `vite build` 0, apertura + visual
verificados en preview :5200). SIN commit: el árbol mezcla trabajo ajeno
sin commitear (169A-1 chat, 169A-4/6/7/8/9/11); coordinar antes de commitear.
Submit/success del modal SIN verificación fiable (pestaña Navegador
compartida con la otra sesión; re-test con tab propia: texto `Solicitud
enviada` + fila en BD + `localStorage["solicitud:borrador"]` limpio).
Procesos DEJADOS CORRIENDO (la otra sesión los usa): backend :3001 y
preview :5200 (build con `VITE_API_URL=http://127.0.0.1:3001`).

## Riesgos
- `src/handlers/mod.rs` tiene cambios sin commitear de 169A-1: editar por
  inserción, no reformatear.
- Fotos huérfanas (subidas sin solicitud): se acepta en v1; LOTE posterior
  con limpieza por job.
