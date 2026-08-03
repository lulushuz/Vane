#Requires -Version 5.1

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'Invoke-VaneAcceptance.ps1')

function Assert-True {
    param(
        [Parameter(Mandatory = $true)][bool]$Condition,
        [Parameter(Mandatory = $true)][string]$Message
    )
    if (-not $Condition) {
        throw $Message
    }
}

$proxy = Protect-EvidenceText '--proxy-socks5=user:password@127.0.0.1:1080'
Assert-True ($proxy -match '<REDACTED>') 'Proxy credentials were not redacted.'
Assert-True ($proxy -notmatch 'password') 'Proxy password remained in evidence.'

$userPath = Protect-EvidenceText (Join-Path $env:USERPROFILE 'private\hostlist.txt')
Assert-True ($userPath -match '<USER_PATH>') 'User profile path was not redacted.'

function New-FirewallRuleFixture {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [string]$DisplayName = $Name,
        [string]$InstanceID = $Name
    )
    return [pscustomobject]@{
        Name = $Name
        DisplayName = $DisplayName
        InstanceID = $InstanceID
    }
}

$currentRuleNames = @(
    'Vane-DNS-vaneinst-dnsinst-r12-AllowUDP',
    'Vane-DNS-vaneinst-dnsinst-r12-AllowTCP',
    'Vane-DNS-vaneinst-dnsinst-r12-UDP53',
    'Vane-DNS-vaneinst-dnsinst-r12-TCP53'
)
$currentRules = @($currentRuleNames | ForEach-Object { New-FirewallRuleFixture $_ })
$discoveredCurrent = @(Select-VaneFirewallRules $currentRules)
Assert-True ($discoveredCurrent.Count -eq 4) 'discovers_current_vane_dns_rule_names: current rules were missed.'
Assert-True (@($discoveredCurrent.Name | Where-Object { $_ -notin $currentRuleNames }).Count -eq 0) 'discovers_current_vane_dns_rule_names: unexpected rules were returned.'
Write-Output 'PASS discovers_current_vane_dns_rule_names'

$legacyRule = New-FirewallRuleFixture 'VaneDNSKillSwitch'
$discoveredLegacy = @(Select-VaneFirewallRules @($legacyRule))
Assert-True ($discoveredLegacy.Count -eq 1) 'discovers_legacy_vane_dns_rule_name: legacy rule was missed.'
Write-Output 'PASS discovers_legacy_vane_dns_rule_name'

$unrelatedRules = @(
    (New-FirewallRuleFixture 'Random-DNS-Block'),
    (New-FirewallRuleFixture 'GoodbyeDPI'),
    (New-FirewallRuleFixture 'Vane-Unrelated')
)
Assert-True (@(Select-VaneFirewallRules $unrelatedRules).Count -eq 0) 'ignores_unrelated_dns_firewall_rules: unrelated rules were accepted.'
Write-Output 'PASS ignores_unrelated_dns_firewall_rules'

$dualMatch = New-FirewallRuleFixture $currentRuleNames[0] $currentRuleNames[0] 'duplicate-instance'
$deduplicated = @(Select-VaneFirewallRules @($dualMatch, $dualMatch))
Assert-True ($deduplicated.Count -eq 1) 'does_not_duplicate_name_and_display_name_match: duplicate rule was returned.'
Write-Output 'PASS does_not_duplicate_name_and_display_name_match'

$shuffledRules = @($currentRules[2], $currentRules[0], $currentRules[3], $currentRules[1])
$sortedNames = @((Select-VaneFirewallRules $shuffledRules).Name)
$expectedNames = @($currentRuleNames | Sort-Object)
Assert-True (($sortedNames -join '|') -eq ($expectedNames -join '|')) 'sorts_rules_deterministically: rules were not sorted by name.'
Write-Output 'PASS sorts_rules_deterministically'

$settings = [ordered]@{
    found = $true
    bypassMode = 'whitelist'
    whitelistDomainCount = 1
    blacklistDomainCount = 0
    dnsCache = $false
    killSwitch = $true
    dnsForwarderEnabled = $true
}
$dns = @([ordered]@{
    interfaceIndex = 1
    addressFamily = 'IPv4'
    serverAddresses = @('1.1.1.1')
})
$before = [ordered]@{ phase = 'before'; dnsAdapters = $dns }
$running = [ordered]@{
    phase = 'running'
    settings = $settings
    processes = @([ordered]@{
        name = 'winws.exe'
        commandLine = 'winws.exe --hostlist=<USER_PATH>\list.txt'
    })
    firewallRules = @(
        [ordered]@{ enabled = 'True'; direction = 'Outbound'; action = 'Block'; remotePort = @('53'); protocol = @('TCP') },
        [ordered]@{ enabled = 'True'; direction = 'Outbound'; action = 'Block'; remotePort = @('53'); protocol = @('UDP') }
    )
    port53Listeners = @([ordered]@{ protocol = 'UDP'; localAddress = '127.0.0.1' })
}
$after = [ordered]@{
    phase = 'after-stop'
    settings = $settings
    processes = @()
    firewallRules = @()
    dnsAdapters = $dns
    port53Listeners = @()
}

$evaluations = @(Get-AutomatedEvaluations @($before, $running, $after))
Assert-True ($evaluations.Count -eq 7) 'The expected evaluation set was not produced.'
Assert-True (@($evaluations | Where-Object { $_.status -eq 'fail' }).Count -eq 0) 'A valid lifecycle fixture failed.'

$running.processes[0].commandLine = 'winws.exe --hostlist-exclude=<USER_PATH>\list.txt'
$negative = @(Get-AutomatedEvaluations @($before, $running, $after))
$pattern = $negative | Where-Object { $_.check -eq 'pattern-argv' } | Select-Object -First 1
Assert-True ($pattern.status -eq 'fail') 'Whitelist mode accepted a blacklist child argument.'

Write-Output 'Vane acceptance harness regression tests passed.'
