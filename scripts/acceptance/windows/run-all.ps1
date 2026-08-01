# Windows 11 Privileged Acceptance Suite for Vane DPI
# Requires Administrator Privileges on a Controlled Disposable VM.

param(
    [switch]$ExecuteOnVM,
    [string]$OutputPath = "artifacts/acceptance/windows-acceptance-results.json"
)

$ErrorActionPreference = "Stop"

function Confirm-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

$results = @{
    timestamp = (Get-Date -Format "o")
    platform = "windows-x64"
    executedOnVm = [bool]$ExecuteOnVM
    overall = "NOT EXECUTED"
    tests = @(
        @{ name = "install"; status = "NOT EXECUTED"; details = "VM execution flag not set" },
        @{ name = "verify-installation"; status = "NOT EXECUTED"; details = "VM execution flag not set" },
        @{ name = "verify-artifacts"; status = "NOT EXECUTED"; details = "VM execution flag not set" },
        @{ name = "test-engine-lifecycle"; status = "NOT EXECUTED"; details = "VM execution flag not set" },
        @{ name = "test-dns-kill-switch"; status = "NOT EXECUTED"; details = "VM execution flag not set" },
        @{ name = "test-optimizer"; status = "NOT EXECUTED"; details = "VM execution flag not set" },
        @{ name = "test-diagnostics"; status = "NOT EXECUTED"; details = "VM execution flag not set" },
        @{ name = "test-tamper-protection"; status = "NOT EXECUTED"; details = "VM execution flag not set" },
        @{ name = "test-uninstall"; status = "NOT EXECUTED"; details = "VM execution flag not set" }
    )
}

if (-not (Confirm-Admin)) {
    Write-Warning "Administrator privileges required. Script must run elevated."
}

if ($ExecuteOnVM) {
    Write-Host "Running Windows Acceptance Suite on VM..." -ForegroundColor Cyan
    # VM Execution logic executes sub-scripts in order
    $scriptDir = $PSScriptRoot
    $allPassed = $true

    foreach ($test in $results.tests) {
        $scriptPath = Join-Path $scriptDir "$($test.name).ps1"
        if (Test-Path $scriptPath) {
            try {
                Write-Host "Executing $($test.name)..." -ForegroundColor Yellow
                & $scriptPath
                $test.status = "PASSED"
                $test.details = "Test passed cleanly"
            } catch {
                $test.status = "FAILED"
                $test.details = $_.Exception.Message
                $allPassed = $false
            }
        }
    }
    $results.overall = if ($allPassed) { "PASSED" } else { "FAILED" }
} else {
    Write-Host "Skipping VM execution (Dry run). Test status set to NOT EXECUTED." -ForegroundColor Yellow
}

$outputDir = Split-Path $OutputPath -Parent
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

$results | ConvertTo-Json -Depth 5 | Set-Content -Path $OutputPath -Encoding UTF8
Write-Host "Acceptance results written to $OutputPath" -ForegroundColor Green
