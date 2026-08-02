$ErrorActionPreference='Stop';$count=0;$bundle=$env:VANE_DIAGNOSTICS_BUNDLE
if(-not $bundle-or-not(Test-Path $bundle)){throw 'VANE_DIAGNOSTICS_BUNDLE must identify a real exported bundle'};$count++
if((Get-Item $bundle).Length-gt 5MB){throw 'Bundle exceeds 5 MiB'};$count++
$text=Get-Content $bundle -Raw;if($text-match 'password=|Bearer |C:\\Users\\|-----BEGIN .*PRIVATE KEY-----'){throw 'Sensitive fixture found'};$count++
[ordered]@{name='test-diagnostics';status='PASSED';assertionCount=$count}|ConvertTo-Json -Compress
