# verify-artifacts.ps1 — Verifies native driver and sidecar binaries
$installDir = "C:\Program Files\Vane"
$required = @("winws.exe", "WinDivert64.sys", "WinDivert.dll", "cygwin1.dll")
foreach ($file in $required) {
    $path = "$installDir\resources\binaries\$file"
    if (-not (Test-Path $path)) {
        throw "Required native artifact missing: $file"
    }
}
Write-Host "All native sidecar artifacts verified."
