' Lanza mantener.ps1 sin abrir consola. La tarea programada MN-Inmobiliaria corre cada 5 min;
' lanzar powershell directamente hacía parpadear una ventana cada vez.
' Revertir: volver a apuntar la tarea a powershell.exe con mantener.ps1.
CreateObject("WScript.Shell").Run "powershell.exe -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File ""C:\Users\Owner\OneDrive\Documentos\area-trabajo\MN-Inmobiliaria\gateway\mantener.ps1""", 0, False
