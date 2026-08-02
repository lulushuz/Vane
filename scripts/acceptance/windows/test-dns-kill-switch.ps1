$ErrorActionPreference='Stop';$count=0;$evidence=$env:VANE_DNS_EVIDENCE
if(-not $evidence-or-not(Test-Path $evidence)){throw 'DNS transaction evidence missing'};$data=Get-Content $evidence -Raw|ConvertFrom-Json
if(-not $data.snapshotRestored-or-not $data.ownedRulesRemoved-or-not $data.foreignRulePreserved){throw 'DNS ownership assertions incomplete'};$count+=3
$snapshot=@(Get-DnsClientServerAddress|Select-Object InterfaceIndex,AddressFamily,ServerAddresses);if(-not $snapshot){throw 'DNS snapshot empty'};$count++
$foreign="VaneAcceptanceForeign-$([guid]::NewGuid())";New-NetFirewallRule -DisplayName $foreign -Direction Outbound -Action Allow -Protocol UDP -RemotePort 5353|Out-Null;$count++
try{if(-not(Get-NetFirewallRule -DisplayName $foreign)){throw 'Foreign firewall fixture missing'};$count++}finally{Remove-NetFirewallRule -DisplayName $foreign -ErrorAction SilentlyContinue}
[ordered]@{name='test-dns-kill-switch';status='PASSED';assertionCount=$count}|ConvertTo-Json -Compress
