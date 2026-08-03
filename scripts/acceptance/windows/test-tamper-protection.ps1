$ErrorActionPreference='Stop';$count=0;$evidence=$env:VANE_TAMPER_EVIDENCE
if(-not $evidence-or-not(Test-Path $evidence)){throw 'Tamper rejection evidence missing'};$count++
$data=Get-Content $evidence -Raw|ConvertFrom-Json;if($data.engineStartRejected-ne $true-or$data.integrityStatus-ne'failed'){throw 'Tamper fail-closed assertions absent'};$count+=2
[ordered]@{name='test-tamper-protection';status='PASSED';assertionCount=$count}|ConvertTo-Json -Compress
