$ErrorActionPreference = 'Stop'
try { [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false) } catch {}

$root = Split-Path -Parent $PSScriptRoot
$fresh = Join-Path $root 'target\release\fastgit.exe'
$buildDir = Join-Path $root 'build'
$installed = Join-Path $buildDir 'fastgit.exe'

$hook = $null
if ([Console]::IsInputRedirected) {
    $payload = [Console]::In.ReadToEnd()
    if ($payload.Trim()) { $hook = $payload | ConvertFrom-Json }
}

function Write-Report([string] $message) {
    if ($hook) { @{ systemMessage = $message } | ConvertTo-Json -Compress }
    else { Write-Output $message }
}

function Sync-BundledConnectors {
    $mirror = Join-Path $buildDir 'connectors'
    $bundled = @()
    foreach ($crate in Get-ChildItem (Join-Path $root 'connectors') -Directory) {
        $dist = Join-Path $crate.FullName 'dist'
        if (-not (Test-Path (Join-Path $dist 'connector.json'))) { continue }
        $bundled += $crate.Name
        $into = Join-Path $mirror $crate.Name
        New-Item -ItemType Directory -Force $into | Out-Null
        Get-ChildItem $into -File |
            Where-Object { -not (Test-Path (Join-Path $dist $_.Name)) } |
            Remove-Item -Force
        Copy-Item (Join-Path $dist '*') $into -Force
    }
    if (Test-Path $mirror) {
        Get-ChildItem $mirror -Directory |
            Where-Object { $bundled -notcontains $_.Name } |
            Remove-Item -Recurse -Force
    }
}

function Get-CompilerErrors([object[]] $log) {
    $lines = @($log | ForEach-Object { "$_" })
    $start = 0
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^error') { $start = $i; break }
    }
    ($lines | Select-Object -Skip $start -First 80) -join "`n"
}

Push-Location $root
try {
    $log = & cmd /c 'cargo build --release --color never 2>&1'
    $exitCode = $LASTEXITCODE
}
finally { Pop-Location }

if ($exitCode -ne 0) {
    $summary = 'FastGit failed to build, so build\fastgit.exe was left as it was.'
    $errors = Get-CompilerErrors $log
    if ($hook -and -not $hook.stop_hook_active) {
        [Console]::Error.WriteLine("$summary Fix these errors:`n$errors")
        exit 2
    }
    Write-Report "$summary`n$errors"
    if ($hook) { exit 0 } else { exit 1 }
}

Sync-BundledConnectors

if ((Test-Path $installed) -and ((Get-FileHash $fresh).Hash -eq (Get-FileHash $installed).Hash)) {
    if (-not $hook) { Write-Report 'build\fastgit.exe is already the latest build.' }
    exit 0
}

$running = @(Get-Process -Name fastgit -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -and ($_.Path -eq $installed) })
$running | Stop-Process -Force
$running | ForEach-Object { $_.WaitForExit(10000) | Out-Null }

New-Item -ItemType Directory -Force $buildDir | Out-Null
for ($attempt = 1; ; $attempt++) {
    try { Copy-Item $fresh $installed -Force; break }
    catch {
        if ($attempt -ge 20) { throw }
        Start-Sleep -Milliseconds 250
    }
}

if ($running.Count -gt 0) {
    Start-Process -FilePath $installed -WorkingDirectory $root
    Write-Report 'FastGit rebuilt and relaunched from build\fastgit.exe.'
}
else {
    Write-Report 'FastGit rebuilt into build\fastgit.exe. It was not running, so it was not started.'
}
exit 0
