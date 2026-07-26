#Requires -Version 5.1

[CmdletBinding()]
param(
    [ValidateSet('Guided', 'Snapshot')]
    [string]$Mode = 'Guided',

    [string]$OutputDirectory = (Join-Path $env:TEMP 'VaneAcceptance'),

    [switch]$CaptureTraffic,

    [switch]$NonInteractive
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:CaptureStarted = $false
$script:CaptureFile = $null
$script:Warnings = [System.Collections.Generic.List[string]]::new()

function Test-IsAdministrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    try {
        $principal = [Security.Principal.WindowsPrincipal]::new($identity)
        return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    }
    finally {
        $identity.Dispose()
    }
}

function Protect-EvidenceText {
    param([AllowNull()][string]$Value)

    if ([string]::IsNullOrWhiteSpace($Value)) {
        return $Value
    }

    $redacted = $Value
    foreach ($root in @($env:USERPROFILE, $env:APPDATA, $env:LOCALAPPDATA)) {
        if (-not [string]::IsNullOrWhiteSpace($root)) {
            $redacted = $redacted.Replace($root, '<USER_PATH>')
        }
    }

    $redacted = [regex]::Replace(
        $redacted,
        '(?i)(--[^\s=]*(?:proxy|socks)[^\s=]*(?:=|\s+))[^\s"]+',
        '$1<REDACTED>'
    )
    $redacted = [regex]::Replace(
        $redacted,
        '(?i)(socks5h?://)(?:[^@\s/]+@)?[^\s/]+',
        '$1<REDACTED>'
    )
    return $redacted
}

function Invoke-EvidenceQuery {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][scriptblock]$Action
    )

    try {
        return & $Action
    }
    catch {
        $message = "${Name}: $($_.Exception.Message)"
        $script:Warnings.Add((Protect-EvidenceText $message))
        return @()
    }
}

function Get-ObjectProperty {
    param(
        [AllowNull()]$InputObject,
        [Parameter(Mandatory = $true)][string]$Name,
        $DefaultValue = $null
    )

    if ($null -eq $InputObject) {
        return $DefaultValue
    }
    if ($InputObject -is [Collections.IDictionary]) {
        if (-not $InputObject.Contains($Name) -or $null -eq $InputObject[$Name]) {
            return $DefaultValue
        }
        return $InputObject[$Name]
    }
    $property = $InputObject.PSObject.Properties[$Name]
    if ($null -eq $property -or $null -eq $property.Value) {
        return $DefaultValue
    }
    return $property.Value
}

function Get-VaneProcesses {
    $names = @('Vane.exe', 'vane-dpi.exe', 'winws.exe', 'winws-x86_64-pc-windows-msvc.exe', 'nfqws.exe')
    $items = Invoke-EvidenceQuery 'Process inventory' {
        Get-CimInstance Win32_Process | Where-Object { $names -contains $_.Name }
    }

    return @($items | ForEach-Object {
        [ordered]@{
            name = $_.Name
            processId = [int]$_.ProcessId
            parentProcessId = [int]$_.ParentProcessId
            executablePath = Protect-EvidenceText $_.ExecutablePath
            commandLine = Protect-EvidenceText $_.CommandLine
        }
    })
}

function Get-WinDivertDrivers {
    $items = Invoke-EvidenceQuery 'WinDivert driver inventory' {
        Get-CimInstance Win32_SystemDriver | Where-Object {
            $_.Name -match 'WinDivert' -or
            $_.DisplayName -match 'WinDivert' -or
            $_.PathName -match 'WinDivert'
        }
    }

    return @($items | ForEach-Object {
        [ordered]@{
            name = $_.Name
            displayName = $_.DisplayName
            state = $_.State
            startMode = $_.StartMode
            pathName = Protect-EvidenceText $_.PathName
        }
    })
}

function Get-VaneFirewallRules {
    $rules = Invoke-EvidenceQuery 'Vane firewall inventory' {
        Get-NetFirewallRule -DisplayName 'VaneDNSKillSwitch' -ErrorAction SilentlyContinue
    }

    return @($rules | ForEach-Object {
        $rule = $_
        $ports = @(Invoke-EvidenceQuery "Firewall port filter $($rule.InstanceID)" {
            $rule | Get-NetFirewallPortFilter
        })
        $addresses = @(Invoke-EvidenceQuery "Firewall address filter $($rule.InstanceID)" {
            $rule | Get-NetFirewallAddressFilter
        })
        [ordered]@{
            displayName = $rule.DisplayName
            enabled = [string]$rule.Enabled
            direction = [string]$rule.Direction
            action = [string]$rule.Action
            profile = [string]$rule.Profile
            protocol = @($ports | ForEach-Object { [string]$_.Protocol })
            remotePort = @($ports | ForEach-Object { [string]$_.RemotePort })
            remoteAddress = @($addresses | ForEach-Object { @($_.RemoteAddress) })
        }
    })
}

function Get-DnsAdapterState {
    $addresses = Invoke-EvidenceQuery 'DNS adapter inventory' {
        Get-DnsClientServerAddress | Sort-Object InterfaceIndex, AddressFamily
    }

    return @($addresses | ForEach-Object {
        [ordered]@{
            interfaceIndex = [int]$_.InterfaceIndex
            interfaceAlias = $_.InterfaceAlias
            addressFamily = [string]$_.AddressFamily
            serverAddresses = @($_.ServerAddresses)
        }
    })
}

function Get-Port53Listeners {
    $tcp = @(Invoke-EvidenceQuery 'TCP port 53 listeners' {
        Get-NetTCPConnection -State Listen -LocalPort 53 -ErrorAction SilentlyContinue
    } | ForEach-Object {
        [ordered]@{
            protocol = 'TCP'
            localAddress = $_.LocalAddress
            localPort = [int]$_.LocalPort
            owningProcess = [int]$_.OwningProcess
        }
    })
    $udp = @(Invoke-EvidenceQuery 'UDP port 53 listeners' {
        Get-NetUDPEndpoint -LocalPort 53 -ErrorAction SilentlyContinue
    } | ForEach-Object {
        [ordered]@{
            protocol = 'UDP'
            localAddress = $_.LocalAddress
            localPort = [int]$_.LocalPort
            owningProcess = [int]$_.OwningProcess
        }
    })
    return @($tcp + $udp)
}

function Get-SettingsEvidence {
    $candidates = @(
        (Join-Path $env:APPDATA 'com.vane.dpi\settings.json'),
        (Join-Path $env:LOCALAPPDATA 'com.vane.dpi\settings.json')
    ) | Select-Object -Unique

    $path = $candidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
    if (-not $path) {
        return [ordered]@{ found = $false; candidates = @($candidates | ForEach-Object { Protect-EvidenceText $_ }) }
    }

    try {
        $file = Get-Item -LiteralPath $path
        if ($file.Length -gt 1MB) {
            throw 'settings.json exceeds the application 1 MiB limit.'
        }

        $outer = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
        $payload = Get-ObjectProperty $outer 'vane-settings'
        if ($payload -is [string]) {
            $payload = $payload | ConvertFrom-Json
        }
        $state = Get-ObjectProperty $payload 'state'
        if ($null -eq $state) {
            throw 'The vane-settings payload has no state object.'
        }

        return [ordered]@{
            found = $true
            path = Protect-EvidenceText $path
            sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
            lastWriteTimeUtc = $file.LastWriteTimeUtc.ToString('o')
            activePresetId = [string](Get-ObjectProperty $state 'activePresetId' '')
            bypassMode = [string](Get-ObjectProperty $state 'bypassMode' '')
            whitelistDomainCount = @(Get-ObjectProperty $state 'whitelistDomains' @()).Count
            blacklistDomainCount = @(Get-ObjectProperty $state 'blacklistDomains' @()).Count
            dnsProtocol = [string](Get-ObjectProperty $state 'dnsProtocol' '')
            dnsCache = [bool](Get-ObjectProperty $state 'dnsCache' $false)
            dnsAdBlock = [bool](Get-ObjectProperty $state 'dnsAdBlock' $false)
            proxyConfigured = -not [string]::IsNullOrWhiteSpace([string](Get-ObjectProperty $state 'proxySocks5' ''))
            killSwitch = [bool](Get-ObjectProperty $state 'killSwitch' $false)
            watchdog = [bool](Get-ObjectProperty $state 'watchdog' $false)
            dnsForwarderEnabled = [bool](Get-ObjectProperty $state 'dnsForwarderEnabled' $false)
            healthCheckTargetCount = @(Get-ObjectProperty $state 'healthCheckTargets' @()).Count
        }
    }
    catch {
        $safeError = Protect-EvidenceText $_.Exception.Message
        $script:Warnings.Add("Settings evidence: $safeError")
        return [ordered]@{
            found = $true
            path = Protect-EvidenceText $path
            readable = $false
            error = $safeError
        }
    }
}

function Get-SystemEvidence {
    $os = Get-CimInstance Win32_OperatingSystem
    return [ordered]@{
        computerNameHash = (ConvertTo-Sha256 $env:COMPUTERNAME)
        osCaption = $os.Caption
        osVersion = $os.Version
        osBuildNumber = $os.BuildNumber
        architecture = $env:PROCESSOR_ARCHITECTURE
        powershellVersion = $PSVersionTable.PSVersion.ToString()
        administrator = Test-IsAdministrator
    }
}

function ConvertTo-Sha256 {
    param([AllowEmptyString()][string]$Value)

    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Value)
        return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '')
    }
    finally {
        $sha.Dispose()
    }
}

function Get-RepositoryEvidence {
    $root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
    $version = $null
    $configPath = Join-Path $root 'src-tauri\tauri.conf.json'
    if (Test-Path -LiteralPath $configPath) {
        $version = (Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json).version
    }

    $commit = $null
    if (Get-Command git -ErrorAction SilentlyContinue) {
        $commit = (& git -C $root rev-parse HEAD 2>$null)
    }
    return [ordered]@{ version = $version; commit = $commit }
}

function Get-EvidenceSnapshot {
    param([Parameter(Mandatory = $true)][string]$Phase)

    return [ordered]@{
        phase = $Phase
        capturedAtUtc = [DateTime]::UtcNow.ToString('o')
        processes = @(Get-VaneProcesses)
        winDivertDrivers = @(Get-WinDivertDrivers)
        firewallRules = @(Get-VaneFirewallRules)
        dnsAdapters = @(Get-DnsAdapterState)
        port53Listeners = @(Get-Port53Listeners)
        settings = Get-SettingsEvidence
    }
}

function Get-AutomatedEvaluations {
    param([Parameter(Mandatory = $true)]$Snapshots)

    $results = [System.Collections.Generic.List[object]]::new()
    $before = $Snapshots | Where-Object { $_.phase -eq 'before' } | Select-Object -First 1
    $running = $Snapshots | Where-Object { $_.phase -eq 'running' } | Select-Object -First 1
    $after = $Snapshots | Where-Object { $_.phase -eq 'after-stop' } | Select-Object -First 1

    if ($null -eq $running -or $null -eq $after) {
        $results.Add([ordered]@{
            check = 'guided-lifecycle'
            status = 'not-evaluated'
            detail = 'Snapshot mode does not include running and after-stop checkpoints.'
        })
        return @($results)
    }

    $mode = [string](Get-ObjectProperty $running.settings 'bypassMode' '')
    $winws = @($running.processes | Where-Object { $_.name -match '^winws' })
    if ($winws.Count -eq 0) {
        $results.Add([ordered]@{ check = 'pattern-argv'; status = 'fail'; detail = 'No running winws process was observed.' })
    }
    elseif ([string]::IsNullOrWhiteSpace($mode)) {
        $results.Add([ordered]@{ check = 'pattern-argv'; status = 'not-evaluated'; detail = 'Persisted Pattern mode was unavailable.' })
    }
    else {
        $commandLine = [string]$winws[0].commandLine
        $hasHostlist = $commandLine -match '(?i)--hostlist(?:=|\s)'
        $hasExclude = $commandLine -match '(?i)--hostlist-exclude(?:=|\s)'
        $patternMatches = switch ($mode) {
            'whitelist' { $hasHostlist -and -not $hasExclude }
            'blacklist' { $hasExclude -and -not $hasHostlist }
            'all' { -not $hasHostlist -and -not $hasExclude }
            default { $false }
        }
        $results.Add([ordered]@{
            check = 'pattern-argv'
            status = $(if ($patternMatches) { 'pass' } else { 'fail' })
            detail = "Observed mode '$mode'; hostlist=$hasHostlist; exclude=$hasExclude."
        })
    }

    $killSwitch = [bool](Get-ObjectProperty $running.settings 'killSwitch' $false)
    $validRules = @($running.firewallRules | Where-Object {
        $_.enabled -eq 'True' -and $_.direction -eq 'Outbound' -and $_.action -eq 'Block' -and
        @($_.remotePort) -contains '53'
    })
    $protocols = @($validRules | ForEach-Object { @($_.protocol) })
    $ruleMatches = if ($killSwitch) {
        $protocols -contains 'TCP' -and $protocols -contains 'UDP'
    }
    else {
        @($running.firewallRules).Count -eq 0
    }
    $results.Add([ordered]@{
        check = 'dns-kill-switch'
        status = $(if ($ruleMatches) { 'pass' } else { 'fail' })
        detail = "Configured=$killSwitch; observed rules=$(@($running.firewallRules).Count); verified protocols=$($protocols -join ',')."
    })

    $forwarderEnabled = [bool](Get-ObjectProperty $running.settings 'dnsForwarderEnabled' $false)
    $loopbackListeners = @($running.port53Listeners | Where-Object { $_.localAddress -eq '127.0.0.1' })
    $hasListener = $loopbackListeners.Count -gt 0
    $listenerMatches = $forwarderEnabled -eq $hasListener
    $results.Add([ordered]@{
        check = 'dns-forwarder-listener'
        status = $(if ($listenerMatches) { 'pass' } else { 'fail' })
        detail = "Configured=$forwarderEnabled; port-53 listener observed=$hasListener."
    })

    $remainingProcesses = @($after.processes)
    $results.Add([ordered]@{
        check = 'process-cleanup'
        status = $(if ($remainingProcesses.Count -eq 0) { 'pass' } else { 'fail' })
        detail = "Vane/winws processes after stop=$($remainingProcesses.Count)."
    })

    $remainingRules = @($after.firewallRules)
    $results.Add([ordered]@{
        check = 'firewall-cleanup'
        status = $(if ($remainingRules.Count -eq 0) { 'pass' } else { 'fail' })
        detail = "Vane kill-switch rules after stop=$($remainingRules.Count)."
    })

    $beforeDns = $before.dnsAdapters | ConvertTo-Json -Depth 6 -Compress
    $afterDns = $after.dnsAdapters | ConvertTo-Json -Depth 6 -Compress
    $results.Add([ordered]@{
        check = 'dns-adapter-restore'
        status = $(if ($beforeDns -eq $afterDns) { 'pass' } else { 'fail' })
        detail = 'Compared the complete IPv4/IPv6 adapter DNS snapshot with the baseline.'
    })

    $results.Add([ordered]@{
        check = 'smart-dns-cache-runtime'
        status = 'not-evaluated'
        detail = 'The persisted toggle is recorded, but cache hits/flushes require structured runtime metrics.'
    })
    return @($results)
}

function Start-EvidenceCapture {
    param([Parameter(Mandatory = $true)][string]$Directory)

    if (-not (Get-Command pktmon.exe -ErrorAction SilentlyContinue)) {
        throw 'pktmon.exe is not available on this Windows installation.'
    }
    $script:CaptureFile = Join-Path $Directory 'traffic.etl'
    # Header-only capture avoids collecting HTTP/TLS payload bytes while retaining flow evidence.
    & pktmon.exe start --capture --pkt-size 128 --file-name $script:CaptureFile | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "pktmon could not start (exit code $LASTEXITCODE)."
    }
    $script:CaptureStarted = $true
}

function Stop-EvidenceCapture {
    param([Parameter(Mandatory = $true)][string]$Directory)

    if (-not $script:CaptureStarted) {
        return
    }

    & pktmon.exe stop | Out-Null
    $script:CaptureStarted = $false
    if ($script:CaptureFile -and (Test-Path -LiteralPath $script:CaptureFile)) {
        $textPath = Join-Path $Directory 'traffic.txt'
        & pktmon.exe etl2txt $script:CaptureFile --out $textPath | Out-Null
        if ($LASTEXITCODE -ne 0) {
            $script:Warnings.Add('pktmon capture succeeded, but ETL-to-text conversion failed.')
        }
    }
}

function Read-Outcome {
    param([Parameter(Mandatory = $true)][string]$Prompt)

    if ($NonInteractive) {
        return 'not-recorded'
    }
    $answer = Read-Host "$Prompt [pass/fail/skip]"
    if ($answer -notin @('pass', 'fail', 'skip')) {
        return 'not-recorded'
    }
    return $answer
}

function Write-MarkdownReport {
    param(
        [Parameter(Mandatory = $true)]$Report,
        [Parameter(Mandatory = $true)][string]$Path
    )

    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.Add('# Vane Windows Acceptance Evidence')
    $lines.Add('')
    $lines.Add("- Session: ``$($Report.sessionId)``")
    $lines.Add("- Started (UTC): ``$($Report.startedAtUtc)``")
    $lines.Add("- Vane version: ``$($Report.repository.version)``")
    $lines.Add("- Commit: ``$($Report.repository.commit)``")
    $lines.Add("- Windows: $($Report.system.osCaption) $($Report.system.osVersion) ($($Report.system.architecture))")
    $lines.Add("- Elevated: ``$($Report.system.administrator)``")
    $lines.Add('')
    $lines.Add('## Operator outcomes')
    $lines.Add('')
    $lines.Add('| Check | Result |')
    $lines.Add('| --- | --- |')
    foreach ($property in $Report.operatorOutcomes.PSObject.Properties) {
        $lines.Add("| $($property.Name) | $($property.Value) |")
    }
    $lines.Add('')
    $lines.Add('## Automated evaluations')
    $lines.Add('')
    $lines.Add('| Check | Status | Detail |')
    $lines.Add('| --- | --- | --- |')
    foreach ($evaluation in $Report.automatedEvaluations) {
        $lines.Add("| $($evaluation.check) | $($evaluation.status) | $($evaluation.detail) |")
    }
    $lines.Add('')
    $lines.Add('## Observed state')
    $lines.Add('')
    $lines.Add('| Phase | Vane/winws processes | WinDivert drivers | Kill-switch rules | Port 53 listeners | Pattern | DNS cache |')
    $lines.Add('| --- | ---: | ---: | ---: | ---: | --- | --- |')
    foreach ($snapshot in $Report.snapshots) {
        $pattern = '-'
        $cache = '-'
        $settingsReadable = Get-ObjectProperty $snapshot.settings 'bypassMode' $null
        if (-not [string]::IsNullOrWhiteSpace([string]$settingsReadable)) {
            $pattern = "$settingsReadable (W:$(Get-ObjectProperty $snapshot.settings 'whitelistDomainCount' 0), B:$(Get-ObjectProperty $snapshot.settings 'blacklistDomainCount' 0))"
            $cache = [string](Get-ObjectProperty $snapshot.settings 'dnsCache' $false)
        }
        $lines.Add("| $($snapshot.phase) | $(@($snapshot.processes).Count) | $(@($snapshot.winDivertDrivers).Count) | $(@($snapshot.firewallRules).Count) | $(@($snapshot.port53Listeners).Count) | $pattern | $cache |")
    }
    $lines.Add('')
    $lines.Add('## Warnings')
    $lines.Add('')
    if (@($Report.warnings).Count -eq 0) {
        $lines.Add('- None.')
    }
    else {
        foreach ($warning in $Report.warnings) {
            $lines.Add("- $warning")
        }
    }
    $lines.Add('')
    $lines.Add('The JSON report is authoritative. Domain values, proxy endpoints, user paths, and the computer name are not included in plaintext.')
    [IO.File]::WriteAllLines($Path, $lines, [Text.UTF8Encoding]::new($false))
}

# Dot-sourcing exposes pure helpers to the regression test without touching system state.
if ($MyInvocation.InvocationName -eq '.') {
    return
}

if ($env:OS -ne 'Windows_NT') {
    throw 'This acceptance harness supports Windows only.'
}
if (-not (Test-IsAdministrator)) {
    throw 'Run this script from an elevated PowerShell session (Run as administrator).'
}
if ($Mode -eq 'Guided' -and $NonInteractive) {
    throw 'Guided mode requires operator checkpoints. Use -Mode Snapshot with -NonInteractive.'
}
if ($CaptureTraffic -and $Mode -ne 'Guided') {
    throw 'Packet capture is available only in Guided mode.'
}

$sessionId = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss') + '-' + ([guid]::NewGuid().ToString('N').Substring(0, 8))
$sessionDirectory = Join-Path $OutputDirectory $sessionId
New-Item -ItemType Directory -Path $sessionDirectory -Force | Out-Null

$snapshots = [System.Collections.Generic.List[object]]::new()
$outcomes = [ordered]@{
    whitelistTargetReachable = 'not-recorded'
    whitelistNonTargetUnaffected = 'not-recorded'
    blacklistTargetExcluded = 'not-recorded'
    tcpTrafficVerified = 'not-recorded'
    quicTrafficVerified = 'not-recorded'
    cleanStopRestoredDns = 'not-recorded'
}

try {
    Write-Host "Evidence session: $sessionId"
    $snapshots.Add((Get-EvidenceSnapshot 'before'))

    if ($Mode -eq 'Guided') {
        if (-not $NonInteractive) {
            Read-Host 'Start Vane, select the intended Pattern/DNS/Advanced settings, start DPI, then press Enter'
        }

        $snapshots.Add((Get-EvidenceSnapshot 'running'))

        if ($CaptureTraffic) {
            Start-EvidenceCapture $sessionDirectory
            Read-Host 'Exercise whitelist/blacklist targets in TCP and QUIC-capable clients, then press Enter to stop capture'
            Stop-EvidenceCapture $sessionDirectory
        }

        $outcomes.whitelistTargetReachable = Read-Outcome 'Whitelist target received DPI processing as expected'
        $outcomes.whitelistNonTargetUnaffected = Read-Outcome 'A non-whitelisted target remained outside Pattern scope'
        $outcomes.blacklistTargetExcluded = Read-Outcome 'A blacklisted target remained outside DPI processing'
        $outcomes.tcpTrafficVerified = Read-Outcome 'TCP behavior matched the selected Pattern'
        $outcomes.quicTrafficVerified = Read-Outcome 'QUIC/UDP 443 behavior matched the selected Pattern'

        if (-not $NonInteractive) {
            Read-Host 'Stop DPI and exit Vane normally, then press Enter'
        }
        $snapshots.Add((Get-EvidenceSnapshot 'after-stop'))
        $outcomes.cleanStopRestoredDns = Read-Outcome 'DNS adapters and Vane firewall rules returned to the expected state'
    }

    $report = [ordered]@{
        schemaVersion = 1
        sessionId = $sessionId
        startedAtUtc = $snapshots[0].capturedAtUtc
        completedAtUtc = [DateTime]::UtcNow.ToString('o')
        mode = $Mode
        packetCaptureRequested = [bool]$CaptureTraffic
        system = Get-SystemEvidence
        repository = Get-RepositoryEvidence
        operatorOutcomes = [pscustomobject]$outcomes
        snapshots = @($snapshots)
        automatedEvaluations = @(Get-AutomatedEvaluations $snapshots)
        warnings = @($script:Warnings)
    }

    $jsonPath = Join-Path $sessionDirectory 'evidence.json'
    $markdownPath = Join-Path $sessionDirectory 'REPORT.md'
    [IO.File]::WriteAllText($jsonPath, ($report | ConvertTo-Json -Depth 12), [Text.UTF8Encoding]::new($false))
    Write-MarkdownReport ([pscustomobject]$report) $markdownPath

    Write-Host "Evidence JSON: $jsonPath"
    Write-Host "Human-readable report: $markdownPath"
}
finally {
    if ($script:CaptureStarted) {
        try {
            Stop-EvidenceCapture $sessionDirectory
        }
        catch {
            Write-Warning "pktmon cleanup failed: $($_.Exception.Message)"
        }
    }
}
