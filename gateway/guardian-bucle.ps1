# Bucle guardián MN-Inmobiliaria (10AA-2): ejecuta `mantener.ps1` al arrancar
# y cada 5 minutos. Lo lanza el acceso directo de la carpeta Inicio (sin
# admin) y queda oculto. Log: C:\tmp\mn-guardian.log
$ErrorActionPreference = "SilentlyContinue"
$Mantener = Join-Path $PSScriptRoot "mantener.ps1"
while ($true) {
  & $Mantener
  Start-Sleep -Seconds 300
}
