param([string]$InstallerPath = $env:VANE_INSTALLER)
$ErrorActionPreference='Stop'; $count=0
if (-not $InstallerPath) { $InstallerPath=(Get-ChildItem 'src-tauri/target/release/bundle/nsis/*setup.exe' -ErrorAction SilentlyContinue | Select-Object -First 1).FullName }
if (-not $InstallerPath -or -not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) { throw 'NSIS installer missing' }; $count++
$sumLine=Get-Content 'artifacts/SHA256SUMS' | Where-Object { $_ -match [regex]::Escape((Split-Path $InstallerPath -Leaf)) }
if (-not $sumLine) { throw 'Installer checksum evidence missing' }
if ((Get-FileHash $InstallerPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne ($sumLine -split '\s+')[0]) { throw 'Installer checksum mismatch' }; $count++
$process=Start-Process -FilePath $InstallerPath -ArgumentList '/S' -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "Installer failed: $($process.ExitCode)" }; $count++
if (-not (Test-Path 'C:\Program Files\Vane' -PathType Container)) { throw 'Install directory missing' }; $count++
[ordered]@{name='install';status='PASSED';assertionCount=$count}|ConvertTo-Json -Compress
