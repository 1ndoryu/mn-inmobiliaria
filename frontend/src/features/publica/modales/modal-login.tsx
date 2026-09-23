import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import type { useModalLogin } from '../../../hooks/publica/modales/use-modal-login';
import { CLASE_ACTIVO, CLASE_BORDE, CLASE_FONDO, CLASE_TEXTO, CLASE_TINTA } from '../disenno';

/* Modal "Entrar al panel": mismo concepto que el modal publicar (cuadrado,
 * tinta, sin sombras, Söhne 400 sin negritas; cierra con overlay o Escape,
 * sin X). Reutiliza el login contra la API: al entrar va a `/admin`. */
export function ModalLogin({ modal }: { modal: ReturnType<typeof useModalLogin> }) {
  const { abierto, cerrar, login } = modal;

  return (
    <Dialog open={abierto} onOpenChange={(a) => !a && cerrar()}>
      <DialogContent
        showCloseButton={false}
        className={`max-h-[92dvh] overflow-y-auto rounded-none border ${CLASE_BORDE} ${CLASE_FONDO} ${CLASE_TINTA} p-6 font-soehne font-normal sm:max-w-2xl`}
      >
        <DialogTitle className="font-soehne text-xl leading-tight font-normal">
          Entrar al panel
        </DialogTitle>
        <form
          className="mt-4 flex flex-col gap-4 font-normal"
          onSubmit={(e) => {
            e.preventDefault();
            void login.ejecutar('entrar');
          }}
        >
          <Campo
            etiqueta="Correo"
            valor={login.email}
            alCambiar={login.setEmail}
            placeholder="admin@ejemplo.com"
            tipo="email"
          />
          <Campo
            etiqueta="Contraseña"
            valor={login.clave}
            alCambiar={login.setClave}
            placeholder="Tu contraseña"
            tipo="password"
          />
          {login.error && (
            <p role="alert" className="text-sm font-normal text-red-800">
              {login.error}
            </p>
          )}
          <button
            type="submit"
            disabled={login.ocupado}
            className={`cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 ${CLASE_TEXTO} disabled:cursor-wait disabled:opacity-60`}
          >
            {login.ocupado ? 'Entrando…' : 'Entrar'}
          </button>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function Campo({
  etiqueta,
  valor,
  alCambiar,
  placeholder,
  tipo = 'text',
}: {
  etiqueta: string;
  valor: string;
  alCambiar: (v: string) => void;
  placeholder: string;
  tipo?: string;
}) {
  return (
    <label className={`flex flex-col gap-1 text-sm font-normal ${CLASE_TINTA}`}>
      {etiqueta}
      <input
        type={tipo}
        value={valor}
        onChange={(e) => alCambiar(e.target.value)}
        placeholder={placeholder}
        className={`w-full rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-2 text-sm font-normal outline-none placeholder:text-black/40`}
      />
    </label>
  );
}
