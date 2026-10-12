# Plan 279A — Extras públicos: candidatos para la ficha

Fecha: 2026-10-12. Estado: **propuesta. Nada se publica** hasta que la usuaria revise la lista, lista por lista.

## Objetivo y alcance

Decidir qué respuestas de /ask (`frontend/src/domain/ficha-ask.ts`) pueden verse en la ficha pública.

- **Dentro:** la lista candidata y el criterio de publicación.
- **Fuera (hoy):** la ficha pública no lee `extras` (`src/repositories/inmueble.rs:27` fuerza `'{}'::JSONB`). La IA sí los lee (`src/handlers/chat.rs:134`); no cambia.

## Criterio propuesto (pendiente de confirmación)

1. Solo se publica lo positivo que agrega valor. Un «no» nunca sale como etiqueta negativa («Sin ascensor» no aparece).
2. Sí/no: se publica únicamente el «Sí».
3. Opciones: solo el valor positivo explícito. «A veces», «Depende del trato» y «No lo sé» no se publican.
4. Texto libre: no se publica sin revisión manual (puede traer teléfonos, direcciones o datos privados).
5. Datos que dependen del edificio (piso): no se publican automáticamente; la usuaria decide caso por caso.
6. Nunca se publica: `precio_minimo` (columna privada, `destino: 'precioMinimo'`) ni la marca `<clave>_nose`.

## Candidatos por lista

Propuesta: **Publicar** = se muestra cuando la respuesta es positiva. **Revisar** = la usuaria decide caso por caso. **Privado** = no sale.

### Comunes (todos los tipos)

| Clave | Pregunta | Tipo | Propuesta | Motivo |
|---|---|---|---|---|
| `punto_referencia` | Punto de referencia cercano | texto | Revisar | Ayuda a ubicar, pero puede revelar la dirección exacta. |
| `negociable` | ¿Precio negociable? | sí/no | **Publicar** (decidido por la usuaria, 2026-10-12) | Recomendación previa: privado, por táctica de venta. La usuaria decidió publicarlo. |
| `precio_minimo` | Mínimo aceptado | decimal | Privado, nunca | Columna aparte; ya es privada. |

### Servicios (todos los tipos)

| Clave | Pregunta | Tipo | Propuesta | Motivo |
|---|---|---|---|---|
| `internet` | ¿Posee internet? | sí/no | Publicar si «Sí» | Valor claro, sobre todo para oficina o teletrabajo. |
| `agua` | ¿Llega el agua? | opciones | Publicar si «Sí» | «A veces» y «No lo sé» no se muestran. |

### Amoblado (viviendas)

| Clave | Pregunta | Tipo | Propuesta | Motivo |
|---|---|---|---|---|
| `amoblado` | ¿Queda amoblado? | opciones | Publicar si «Sí» | Valor claro. «Depende del trato» no se publica. |

### Apartamento

| Clave | Pregunta | Tipo | Propuesta | Motivo |
|---|---|---|---|---|
| `piso` | ¿En qué piso queda? (0 = planta baja) | entero | Revisar | Planta baja: a veces suma (acceso, sin escaleras) y a veces resta (seguridad, humedad). Último piso: suele sumar (vista, ventilación), pero depende del edificio. No automático. |
| `ascensor` | ¿El edificio tiene ascensor? | sí/no | Publicar si «Sí» | Valor claro, sobre todo en piso alto. |
| `areas_comunes` | Áreas comunes | texto | Revisar | Valor alto (piscina, gimnasio), pero texto libre. |

### Townhouse

| Clave | Pregunta | Tipo | Propuesta | Motivo |
|---|---|---|---|---|
| `piso` | ¿Cuántas plantas tiene? | entero | Revisar | Dato neutro («2 plantas»). Mostrar como dato, no como ventaja. |
| `conjunto_cerrado` | ¿Conjunto cerrado con vigilancia? | sí/no | Publicar si «Sí» | Valor claro. |
| `areas_comunes` | Áreas comunes del conjunto | texto | Revisar | Igual que apartamento. |

### Casa

| Clave | Pregunta | Tipo | Propuesta | Motivo |
|---|---|---|---|---|
| `conjunto_cerrado` | ¿Conjunto cerrado con vigilancia? | sí/no | Publicar si «Sí» | Valor claro. |
| `ampliacion` | ¿Tiene ampliación posible? | sí/no | Publicar si «Sí» | Valor claro para familias que crecen. |
| `areas_externas` | Patio, terraza, parrillera… | texto | Revisar | Valor, pero texto libre. |

### Local

| Clave | Pregunta | Tipo | Propuesta | Motivo |
|---|---|---|---|---|
| `frente_metros` | Metros de frente (vidriera) | decimal | Revisar | Dato neutro: «8 m» sin contexto no dice si es bueno. |
| `bano` | ¿Tiene baño propio? | sí/no | Publicar si «Sí» | Valor claro para comercio. |
| `deposito` | ¿Depósito o trastienda? | sí/no | Publicar si «Sí» | Valor claro. |

### Terreno

| Clave | Pregunta | Tipo | Propuesta | Motivo |
|---|---|---|---|---|
| `frente_metros` | Metros de frente | decimal | Revisar | Igual que local. |
| `cercado` | ¿Cercado o amojonado? | sí/no | Publicar si «Sí» | Valor claro. |
| `servicios` | Servicios que llegan | texto | Revisar | Agua, luz, vialidad: valor alto, texto libre. |

## Resumen para la revisión

- **Publicar si positivo (candidatos claros):** `internet`, `agua` (solo «Sí»), `amoblado` (solo «Sí»), `ascensor`, `conjunto_cerrado`, `ampliacion`, `bano`, `deposito`, `cercado`.
- **Revisar caso por caso:** `punto_referencia`, `piso` (apartamento y townhouse), `areas_comunes`, `areas_externas`, `frente_metros`, `servicios`.
- **Decidido por la usuaria (2026-10-12):** `negociable` se publica. «A veces» en agua sigue sin mostrarse.
- **Privado:** `precio_minimo` (siempre).

## Fases si se aprueba

1. Confirmar el criterio (puntos 1–6) y la lista de la usuaria.
2. Filtro en backend para la ficha pública: solo claves aprobadas y solo valores positivos. Nunca se envían claves privadas ni `*_nose`.
3. Prueba funcional: una ficha con respuestas mixtas (positivas, «No», «A veces», «No lo sé») y comprobar qué ve la página pública.

## Estado y próximo paso

- Propuesta escrita 2026-10-12. Sin publicar.
- **Pendiente de la usuaria:** revisar la tabla, lista por lista, y confirmar el criterio.
- **Pendiente de implementación:** tras esa aprobación (fases 2–3).
