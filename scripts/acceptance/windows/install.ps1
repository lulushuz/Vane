# install.ps1 — Silent installation of Vane NSIS package on controlled VM
param([string]$InstallerPath = "artifacts/release-candidate/2.1.4/windows-x64-unsigned/Vane-2.1.4-setup.exe")

if (-not (Test-Path $InstallerPath)) {
    throw "Installer binary not found at $InstallerPath"
}
Write-Host "Installing Vane silently from $InstallerPath..."
Start-Process -FilePath $InstallerPath -ArgumentList "/S" -Wait
Write-Host "Installation process completed."
