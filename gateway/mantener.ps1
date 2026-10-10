# Guardián MN-Inmobiliaria (10AA-2): mantiene backend :3110 y gateway :3102
# siempre en pie. Lo llama la tarea programada `MN-Inmobiliaria` al iniciar
# sesión y cada 5 minutos. Solo actúa si el puerto no responde; nunca toca
# procesos ajenos (filtra por línea de comando propia). Log: C:\tmp\mn-guardian.log
$ErrorActionPreference = "SilentlyContinue"
$Repo = "C:\Users\Owner\OneDrive\Documentos\area-trabajo\MN-Inmobiliaria"
$Log = "C:\tmp\mn-guardian.log"

function Nota($m) {
  "[$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')] $m" | Out-File -FilePath $Log -Append -Encoding utf8
}

function PuertoAbierto($p) {
  $c = New-Object Net.Sockets.TcpClient
  try {
    $iar = $c.BeginConnect("127.0.0.1", $p, $null, $null)
    return $iar.AsyncWaitHandle.WaitOne(3000) -and $c.Connected
  } finally { $c.Close() }
}

function Arrancar($nombre, $puerto, $dir, $cmd, $logSalida) {
  if (PuertoAbierto $puerto) { return }
  # ¿Proceso propio huérfano (puerto caído pero exe vivo)? Solo mato si su
  # línea de comando es exactamente la mía; jamás por nombre solo.
  Get-CimInstance Win32_Process -Filter "Name='node.exe'" | Where-Object {
    $_.CommandLine -like "*$cmd*"
  } | ForEach-Object {
    Nota "$nombre`: proceso huérfano PID $($_.ProcessId), lo detengo"
    Stop-Process -Id $_.ProcessId -Force
  }
  Start-Sleep -Seconds 2
  if (PuertoAbierto $puerto) { return }
  Nota "$nombre`: puerto $puerto caído, arrancando ($cmd)"
  # `cmd /c` con redirección: el log lo escribe el propio shell, sin jobs
  # colgados ni handles heredados.
  $psi = New-Object Diagnostics.ProcessStartInfo
  $psi.FileName = "cmd.exe"
  $psi.Arguments = "/c `"node $cmd >> `"$logSalida`" 2>&1`""
  $psi.WorkingDirectory = $dir
  $psi.UseShellExecute = $false
  $psi.CreateNoWindow = $true
  $p = [Diagnostics.Process]::Start($psi)
  Nota "$nombre`: arrancado PID $($p.Id)"
}

Arrancar "backend" "3110" "$Repo" "scripts/run-with-db.mjs run" "C:\tmp\mn-backend.log"
Arrancar "gateway" "3102" "$Repo\gateway" "src/index.mjs --gateway-mn" "C:\tmp\gateway.log"
