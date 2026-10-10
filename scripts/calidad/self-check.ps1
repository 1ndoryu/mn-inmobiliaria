param(
    [string]$TareaId = ""
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

function Write-ChecklistSection {
    param(
        [string]$Title,
        [string[]]$Items
    )

    Write-Host "[self-check] $Title"
    foreach ($item in $Items) {
        Write-Host "[self-check] [ ] $item"
    }
}

function Invoke-Validation {
    param(
        [string]$Label,
        [scriptblock]$Action
    )

    Write-Host "[self-check] Ejecutando $Label..."
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "Fallo la validacion: $Label"
    }
}

$rules = @(
    '[OBLIGATORIA] 0. Leer roadmap completo y aislar una tarea por ciclo',
    '[OBLIGATORIA] 1. Autonomia total sin confirmaciones triviales',
    '[OBLIGATORIA] 2. Resolver la causa raiz, no aplicar parches superficiales',
    '[OBLIGATORIA] 3. Editar por bloques coherentes y validar al cierre',
    '[OBLIGATORIA] 4. Corregir desorden visible de bajo riesgo en archivos tocados',
    '[OBLIGATORIA] 5. Mantener seguridad en inputs, queries y secrets',
    '[OBLIGATORIA] 6. No dejar fallos silenciosos ni operaciones criticas sin retorno',
    '[OBLIGATORIA] 7. Evitar roundtrips y patrones N+1',
    '[OBLIGATORIA] 8. Mantener SRP, limites de tamano y separacion por dominio',
    '[OBLIGATORIA] 9. Respetar naming, CSS tokenizado y referencias existentes',
    '[OBLIGATORIA] 10. Dejar comentarios y lecciones si hubo aprendizaje nuevo',
    '[OBLIGATORIA] 11. Ejecutar validaciones del stack y corregir errores reportados',
    '[OBLIGATORIA] 12. Hacer staging explicito y commit por bloque',
    '[CONDICIONAL] 13. Usar flujo PowerShell+SSH/base64 solo cuando aplique',
    '[CONDICIONAL] 14. Justificar sentinel-disable-file si se usa',
    '[OBLIGATORIA] 15. Verificar responsive en tareas UI',
    '[OBLIGATORIA] 16. Releer roadmap al cerrar la tarea',
    '[CONDICIONAL] 17. Evaluar si algo reusable debe ir a glory-rs',
    '[OBLIGATORIA] 18. Mejorar el proceso si falta tooling critico'
)

$flow = @(
    '[OBLIGATORIA] Paso 1. Leer roadmap y planes activos',
    '[OBLIGATORIA] Paso 2. Ejecutar un bloque completo antes de pasar al siguiente',
    '[OBLIGATORIA] Paso 3. Validar y corregir errores del stack',
    '[OBLIGATORIA] Paso 4. Testear la funcionalidad afectada',
    '[OBLIGATORIA] Paso 5. Archivar la tarea completada',
    '[OBLIGATORIA] Paso 6. Actualizar o crear documentacion si se toco funcionalidad',
    '[CONDICIONAL] Paso 7. Registrar prevencion si Code Sentinel puede detectarlo',
    '[CONDICIONAL] Paso 8. Implementar prevenciones pendientes si existen',
    '[OBLIGATORIA] Paso 9. Commit final y sincronizacion con remoto',
    '[OBLIGATORIA] Paso 10. Releer roadmap y seguir el ciclo hasta vaciar pendientes'
)

Write-Host "[self-check] Proyecto: $projectRoot"
if ($TareaId) {
    Write-Host "[self-check] TareaId: $TareaId"
}

Write-ChecklistSection -Title 'Reglas del protocolo' -Items $rules
Write-ChecklistSection -Title 'Pasos del flujo' -Items $flow

Push-Location $projectRoot
try {
    if (Test-Path (Join-Path $projectRoot 'package.json')) {
        if (Test-Path (Join-Path $projectRoot 'Cargo.toml')) {
            Invoke-Validation -Label 'npm run check:back' -Action { npm run check:back }
            Invoke-Validation -Label 'npm run test' -Action { npm run test }
        }

        if (Test-Path (Join-Path $projectRoot 'frontend\package.json')) {
            Invoke-Validation -Label 'npm run check:front' -Action { npm run check:front }
        }

        if (Test-Path (Join-Path $projectRoot 'scripts\calidad\check-roadmap.mjs')) {
            Write-Host '[self-check] Ejecutando npm run roadmap...'
            npm run roadmap
            $roadmapExitCode = $LASTEXITCODE
            if ($roadmapExitCode -gt 1) {
                throw 'Fallo la validacion del roadmap watcher'
            }

            if ($roadmapExitCode -eq 1) {
                Write-Host '[self-check] El roadmap aun tiene tareas pendientes; continuar el ciclo es esperado.' -ForegroundColor Yellow
            } else {
                Write-Host '[self-check] El roadmap no reporta tareas pendientes.' -ForegroundColor Green
            }
        }
    }

    Write-Host '[self-check] Validacion completada.' -ForegroundColor Green
    exit 0
} catch {
    Write-Error "[self-check] $($_.Exception.Message)"
    exit 1
} finally {
    Pop-Location
}
