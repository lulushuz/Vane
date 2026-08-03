$ErrorActionPreference='Stop';$count=0;$root='C:\Program Files\Vane'
$exe=Get-ChildItem $root -Filter 'Vane.exe' -Recurse | Select-Object -First 1
if(-not $exe){throw 'Vane executable missing'};$count++
foreach($name in @('winws-x86_64-pc-windows-msvc.exe','WinDivert64.sys','WinDivert.dll','cygwin1.dll')){if(-not(Get-ChildItem $root -Filter $name -Recurse)){throw "Missing $name"};$count++}
$process=Start-Process $exe.FullName -PassThru; Start-Sleep -Seconds 2
if($process.HasExited){throw 'Application failed to remain running'};$count++;Stop-Process -Id $process.Id -Force
[ordered]@{name='verify-installation';status='PASSED';assertionCount=$count}|ConvertTo-Json -Compress
