/* [18AA-6] `nombre_corto`: cómo se cita el inmueble en el chat del Marketplace
 * («Apart. en Resd. Kama Merú»). Antes lo deducía el código en cada respuesta a
 * partir del título y salían sujetos como «el apartamento planta baja con patio
 * en Residencias Kama Merú», que suenan a ficha y no a conversación. Ahora es un
 * dato del inmueble: se rellena al darlo de alta y se edita en la ficha. */
ALTER TABLE inmuebles
  ADD COLUMN nombre_corto TEXT NOT NULL DEFAULT '';

/* Tres datos venían tecleados a mano y el nombre corto los copia tal cual. Se
 * corrigen aquí para que el relleno de abajo dé el nombre bueno también en el
 * servidor, sin un paso manual aparte. */
UPDATE inmuebles SET ubicacion = 'Alta Vista' WHERE btrim(ubicacion) = 'Alta vista';
UPDATE inmuebles SET residencia = 'Las Orquídeas' WHERE btrim(residencia) = 'Las Orquideas';
UPDATE inmuebles SET residencia = 'Villa Sol Suites, Lechería'
 WHERE btrim(residencia) = 'Villa Sol Suites';

/* Relleno de las filas que ya existían, con la regla que usaba el código: tipo
 * abreviado («apartamento» → «Apart.», el resto tal cual) + lugar corto
 * («Residencias» → «Resd.», sin el «Edificio»/«Edif.» de delante). El lugar es
 * la residencia tal cual —con su coma, si la lleva—; sin residencia, el primer
 * tramo de la ubicación, porque el resto es la ciudad y en un chat sobra.
 * Esto es una foto de la regla en la fecha de la migración, no una regla viva:
 * a partir de aquí el valor se edita en la ficha del inmueble. */
UPDATE inmuebles SET nombre_corto =
  CASE WHEN btrim(tipo) = '' THEN 'Inmueble'
       WHEN lower(btrim(tipo)) = 'apartamento' THEN 'Apart.'
       ELSE upper(left(btrim(tipo), 1)) || substr(btrim(tipo), 2) END
  || ' en ' ||
  COALESCE((
    SELECT string_agg(
             CASE WHEN lower(p) = 'residencias' THEN 'Resd.' ELSE p END,
             ' ' ORDER BY ord)
      FROM unnest(string_to_array(
             CASE WHEN btrim(residencia) <> '' THEN btrim(residencia)
                  ELSE split_part(ubicacion, ',', 1) END, ' '))
           WITH ORDINALITY AS t(p, ord)
     WHERE lower(p) NOT IN ('edificio', 'edif.') AND p <> ''
  ), '');
