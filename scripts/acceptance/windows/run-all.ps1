param([switch]$ExecuteOnVM, [string]$OutputPath = "artifacts/acceptance/windows-acceptance-results.json")
$ErrorActionPreference = 'Stop'
function Test-Administrator {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = [Security.Principal.WindowsPrincipal]::new($identity)
  $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}
$names = @('install','verify-installation','verify-artifacts','test-engine-lifecycle','test-dns-kill-switch','test-optimizer','test-diagnostics','test-tamper-protection','test-uninstall')
$result = [ordered]@{ schemaVersion=1; timestamp=(Get-Date).ToUniversalTime().ToString('o'); platform='windows-x64'; executedOnVm=[bool]$ExecuteOnVM; overall='NOT EXECUTED'; assertionCount=0; tests=@() }
if (-not $ExecuteOnVM) {
  $result.tests = @($names | ForEach-Object { [ordered]@{name=$_;status='NOT EXECUTED';assertionCount=0} })
} else {
  if (-not (Test-Administrator)) { throw 'Windows acceptance requires an elevated Administrator session.' }
  $allPassed = $true
  foreach ($name in $names) {
    $script = Join-Path $PSScriptRoot "$name.ps1"
    if (-not (Test-Path -LiteralPath $script -PathType Leaf)) {
      $result.tests += [ordered]@{name=$name;status='FAILED';assertionCount=0;error='Missing acceptance script'}; $allPassed=$false; continue
    }
    try {
      $raw = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script 2>&1
      $exitCode = $LASTEXITCODE
      if ($exitCode -ne 0) { throw "Subscript exit code $exitCode: $raw" }
      $parsed = $raw | Select-Object -Last 1 | ConvertFrom-Json
      if ($parsed.status -ne 'PASSED' -or [int]$parsed.assertionCount -le 0) { throw 'Subscript returned no verified assertions.' }
      $result.assertionCount += [int]$parsed.assertionCount
      $result.tests += $parsed
    } catch {
      $allPassed=$false; $result.tests += [ordered]@{name=$name;status='FAILED';assertionCount=0;error=$_.Exception.Message}
    }
  }
  $result.overall = if ($allPassed -and $result.assertionCount -gt 0) {'PASSED'} else {'FAILED'}
}
$parent = Split-Path -Parent $OutputPath; if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
$result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutputPath -Encoding utf8
if ($ExecuteOnVM -and $result.overall -ne 'PASSED') { exit 1 }
