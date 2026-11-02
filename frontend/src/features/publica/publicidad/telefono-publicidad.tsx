import { useState } from 'react';
import { guardarTelefonoAsesor, telefonoAsesor } from '../../../platform/telefono-asesor';
import { CLASE_BORDE } from '../disenno';
import { BOTON_PUBLICIDAD, CAJA_PUBLICIDAD } from './estilos-publicidad';

/* Teléfono global de las imágenes: cada asesor pone el suyo y se guarda en
 * este navegador. Vacío = el de la empresa. `alGuardar` redibuja las imágenes. */
export function TelefonoPublicidad({ alGuardar }: { alGuardar: () => void }) {
  const [valor, setValor] = useState(telefonoAsesor);
  const [mensaje, setMensaje] = useState<string | null>(null);

  const guardar = () => {
    if (!guardarTelefonoAsesor(valor)) {
      setMensaje('No se pudo guardar en este navegador.');
      return;
    }
    setValor(telefonoAsesor());
    setMensaje('Teléfono guardado en este navegador.');
    alGuardar();
  };

  return (
    <div className={`${CAJA_PUBLICIDAD} flex flex-wrap items-end gap-3`}>
      <label className="flex flex-col gap-1">
        Tu teléfono en las imágenes
        <input
          className={`w-64 border ${CLASE_BORDE} bg-transparent px-2 py-1`}
          value={valor}
          maxLength={40}
          onChange={(e) => setValor(e.target.value)}
        />
      </label>
      <button type="button" className={BOTON_PUBLICIDAD} onClick={guardar}>
        Guardar
      </button>
      {mensaje && <p>{mensaje}</p>}
    </div>
  );
}
