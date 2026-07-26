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
