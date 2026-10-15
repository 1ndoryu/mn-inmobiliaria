# Guardián MN-Inmobiliaria (10AA-2 y 15AA-6): mantiene el gateway de WhatsApp
# (aquí, :3102) y su puente con la 89 siempre en pie. Lo llama la tarea
# programada `MN-Inmobiliaria` al iniciar sesión y cada 5 minutos. Solo actúa
# si el puerto no responde; nunca toca procesos ajenos (filtra por línea de
# comando propia). Log: C:\tmp\mn-guardian.log
#
# [15AA-6] MN corre en la 89: el backend de verdad es el de la 89 y su base es
# la que lee el panel. El gateway se queda aquí (tiene las sesiones de WhatsApp
# y no viajan) y habla con la 89 por dos reenvíos del mismo ssh:
#   -L 33110:127.0.0.1:3110  entrante: el gateway avisa al backend de la 89
#   -R 3102:127.0.0.1:3102   saliente: la 89 alcanza el gateway en su 127.0.0.1
#                            (envíos del panel y descarga de medios)
# El backend local ya no se levanta: escribiría en la base del laptop, que no
# es la que ve el panel.
$ErrorActionPreference = "SilentlyContinue"
$Repo = "C:\Users\Owner\OneDrive\Documentos\area-trabajo\MN-Inmobiliaria"
$Ssh = "C:\Windows\System32\OpenSSH\ssh.exe"
$Log = "C:\tmp\mn-guardian.log"
$LogTunel = "C:\tmp\mn-tunel.log"
# ControlMaster=no: el PID de este ssh es el puente; con multiplexado delegaría
# y saldría. ExitOnForwardFailure: si un reenvío no se puede abrir, se cae en
# vez de quedarse a medias (y lo detectamos por el PID que falta).
$Tunel = "-N -o ControlMaster=no -o BatchMode=yes -o ExitOnForwardFailure=yes -o ConnectTimeout=10 -o ServerAliveInterval=30 -o ServerAliveCountMax=3 -L 33110:127.0.0.1:3110 -R 3102:127.0.0.1:3102 dev89"

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

# El puente es un ssh propio: si el enlace muere, ssh sale solo
# (ServerAliveCountMax) o deja de estar en la lista. Que exista es señal
# suficiente; el PID identifica el nuestro por su línea de comando.
function TunelPropio {
  @(Get-CimInstance Win32_Process -Filter "Name='ssh.exe'" | Where-Object {
    $_.CommandLine -like "*33110:127.0.0.1:3110*"
  })
}

if ((TunelPropio).Count -eq 0) {
  Nota "puente: caído, arrancando"
  $psi = New-Object Diagnostics.ProcessStartInfo
  $psi.FileName = "cmd.exe"
  $psi.Arguments = "/c `"$Ssh $Tunel >> `"$LogTunel`" 2>&1`""
  $psi.WorkingDirectory = $Repo
  $psi.UseShellExecute = $false
  $psi.CreateNoWindow = $true
  $p = [Diagnostics.Process]::Start($psi)
  Nota "puente: arrancado PID $($p.Id)"
  Start-Sleep -Seconds 3
  # El reenvío local solo contesta si llegó hasta el backend de la 89: si no,
  # la 89 está parada o el enlace se cayó al nacer (mira C:\tmp\mn-tunel.log).
  if (-not (PuertoAbierto 33110)) { Nota "puente: aviso, 127.0.0.1:33110 no respondió" }
}

Arrancar "gateway" "3102" "$Repo\gateway" "src/index.mjs --gateway-mn" "C:\tmp\gateway.log"
