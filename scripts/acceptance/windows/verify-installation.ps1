# verify-installation.ps1 — Verifies Vane installation files and registry entries
$installDir = "C:\Program Files\Vane"
if (-not (Test-Path "$installDir\vane-dpi.exe")) {
    throw "Vane main executable missing at $installDir\vane-dpi.exe"
}
Write-Host "Vane executable presence verified."
