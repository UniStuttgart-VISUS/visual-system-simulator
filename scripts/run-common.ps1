$ErrorActionPreference = 'Stop'

function Invoke-Native {
    param([string]$Description, [scriptblock]$Command)

    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed with exit code $LASTEXITCODE"
    }
}

function Assert-Tool {
    param([string]$Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Missing required tool: $Name"
    }
}

function Invoke-VssActions {
    param(
        [string[]]$Actions,
        [System.Collections.IDictionary]$Handlers,
        [scriptblock]$Usage,
        [string]$WorkingDirectory = ''
    )

    if ($Actions.Count -eq 0) {
        & $Usage
        return
    }

    foreach ($action in $Actions) {
        if (-not $Handlers.Contains($action)) {
            & $Usage
            throw "Unknown action: $action"
        }
    }

    if ($WorkingDirectory) { Push-Location $WorkingDirectory }
    try {
        foreach ($action in $Actions) {
            & $Handlers[$action]
        }
        Write-Host 'Done.'
    }
    finally {
        if ($WorkingDirectory) { Pop-Location }
    }
}
