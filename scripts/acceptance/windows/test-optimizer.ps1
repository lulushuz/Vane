$ErrorActionPreference='Stop';$count=0;$evidence=$env:VANE_OPTIMIZER_EVIDENCE
if(-not $evidence-or-not(Test-Path $evidence)){throw 'Optimizer session evidence missing'};$count++
$data=Get-Content $evidence -Raw|ConvertFrom-Json;if(-not $data.originalStateRestored){throw 'Original state was not verified restored'};$count++
if(-not $data.configRevision-or-not $data.configFingerprint){throw 'Revision/fingerprint evidence missing'};$count++
[ordered]@{name='test-optimizer';status='PASSED';assertionCount=$count}|ConvertTo-Json -Compress
