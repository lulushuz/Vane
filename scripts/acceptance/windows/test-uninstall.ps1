$ErrorActionPreference='Stop';$count=0;$uninstaller=Get-ChildItem 'C:\Program Files\Vane' -Filter 'uninstall*.exe' -Recurse|Select-Object -First 1
if(-not $uninstaller){throw 'Uninstaller missing'};$count++;$p=Start-Process $uninstaller.FullName -ArgumentList '/S' -Wait -PassThru;if($p.ExitCode-ne 0){throw 'Uninstall failed'};$count++
if(Get-Process -Name winws* -ErrorAction SilentlyContinue){throw 'Owned engine remains'};$count++
[ordered]@{name='test-uninstall';status='PASSED';assertionCount=$count}|ConvertTo-Json -Compress
