$ErrorActionPreference='Stop';$count=0;$evidence=$env:VANE_LIFECYCLE_EVIDENCE
if(-not $evidence-or-not(Test-Path $evidence)){throw 'Lifecycle IPC evidence missing'};$data=Get-Content $evidence -Raw|ConvertFrom-Json
if(-not $data.ownedPid-or-not $data.stopVerified-or-not $data.foreignProcessPreserved-or $data.restartedAfterUserStop){throw 'Lifecycle assertions incomplete'};$count+=4
$foreign=Start-Process powershell.exe -ArgumentList '-NoProfile','-Command','Start-Sleep 30' -PassThru;$count++
try{if(-not(Get-Process -Id $foreign.Id -ErrorAction SilentlyContinue)){throw 'Foreign fixture not alive'};$count++;$owned=Get-Process -Name winws* -ErrorAction SilentlyContinue;if(@($owned).Count-ne 0){throw 'Engine lifecycle requires a clean precondition'};$count++}finally{Stop-Process -Id $foreign.Id -Force -ErrorAction SilentlyContinue}
[ordered]@{name='test-engine-lifecycle';status='PASSED';assertionCount=$count}|ConvertTo-Json -Compress
