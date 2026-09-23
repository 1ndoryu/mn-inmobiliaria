import { Building2, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useLogin } from '@/hooks/sesion/use-login';

// Puerta del panel: login contra la API. Si aún no hay usuarios, el mismo
// formulario crea la cuenta propietaria inicial (el backend cierra el
// registro con 403 en cuanto existe una).
export function PantallaLogin({ alEntrar }: { alEntrar: (email: string) => void }) {
  const { email, clave, ocupado, error, setEmail, setClave, ejecutar } = useLogin(alEntrar);

  return (
    <div className="flex min-h-dvh items-center justify-center bg-background p-4">
      <form
        className="w-full max-w-sm space-y-4 rounded-lg border p-6"
        onSubmit={(e) => {
          e.preventDefault();
          void ejecutar('entrar');
        }}
      >
        <div className="flex items-center gap-2">
          <span className="flex h-9 w-9 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <Building2 className="h-5 w-5" />
          </span>
          <div>
            <p className="text-base font-bold">Inmobiliaria</p>
            <p className="text-xs text-muted-foreground">Panel de administración</p>
          </div>
        </div>
        <div className="space-y-2">
          <label className="text-sm font-medium" htmlFor="login-email">
            Correo
          </label>
          <Input
            id="login-email"
            type="email"
            autoComplete="username"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="admin@ejemplo.com"
          />
        </div>
        <div className="space-y-2">
          <label className="text-sm font-medium" htmlFor="login-clave">
            Contraseña
          </label>
          <Input
            id="login-clave"
            type="password"
            autoComplete="current-password"
            value={clave}
            onChange={(e) => setClave(e.target.value)}
          />
        </div>
        {error && (
          <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </p>
        )}
        <Button type="submit" className="w-full" disabled={ocupado}>
          {ocupado && <Loader2 className="h-4 w-4 animate-spin" />} Entrar
        </Button>
        <Button type="button" variant="outline" className="w-full" disabled={ocupado} onClick={() => void ejecutar('registrar')}>
          Crear cuenta propietaria inicial
        </Button>
        <p className="text-xs text-muted-foreground">
          El registro solo funciona una vez: si ya hay usuarios, la API lo rechaza y hay que entrar.
        </p>
      </form>
    </div>
  );
}
