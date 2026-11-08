param(
    [Parameter(Mandatory)] [string] $Tag,
    [string] $Repository = 'Sulexa/SterneGit',
    [string] $Out = 'dist',
    [string] $Feed = "https://github.com/$Repository/releases/latest/download/latest.json",
    [string] $DownloadBase = "https://github.com/$Repository/releases/download/$Tag",
    [string] $NotesPage = "https://github.com/$Repository/releases/tag/$Tag",
    [string] $Iscc = ''
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Invoke-Native([string] $what, [scriptblock] $command) {
    & $command
    if ($LASTEXITCODE -ne 0) { throw "$what failed with exit code $LASTEXITCODE." }
}

function Get-CargoVersion {
    $manifest = Get-Content (Join-Path $root 'Cargo.toml') -Raw
    if ($manifest -notmatch '(?ms)^\[package\][^\[]*?^version\s*=\s*"([^"]+)"') {
        throw 'Cargo.toml has no [package] version.'
    }
    $Matches[1]
}

function Find-Iscc {
    if ($Iscc) { return $Iscc }
    $command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    $default = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'
    if (Test-Path $default) { return $default }
    throw 'Inno Setup 6 (ISCC.exe) was not found.'
}

function Build-Release {
    $env:STERNEGIT_UPDATE_FEED = $Feed
    Push-Location $root
    try {
        $messages = & cargo build --release --locked --message-format=json-render-diagnostics
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE." }
    }
    finally { Pop-Location }
    $script = $messages |
        ForEach-Object { $_ | ConvertFrom-Json } |
        Where-Object { $_.reason -eq 'build-script-executed' -and $_.package_id -match '(^|[#/ ])sternegit[@ ]' } |
        Select-Object -Last 1
    if (-not $script) { throw "cargo didn't report sternegit's build script output folder." }
    [pscustomobject]@{
        Exe  = Join-Path $root 'target\release\sternegit.exe'
        Icon = Join-Path $script.out_dir 'sternegit.ico'
    }
}

function New-Staging([string] $exe, [string] $staging) {
    if (Test-Path $staging) { Remove-Item $staging -Recurse -Force }
    New-Item -ItemType Directory -Force $staging | Out-Null
    Copy-Item $exe $staging
    foreach ($crate in Get-ChildItem (Join-Path $root 'connectors') -Directory) {
        $dist = Join-Path $crate.FullName 'dist'
        if (-not (Test-Path (Join-Path $dist 'connector.json'))) { continue }
        $into = Join-Path $staging "connectors\$($crate.Name)"
        New-Item -ItemType Directory -Force $into | Out-Null
        Copy-Item (Join-Path $dist '*') $into
    }
}

function New-PortableZip([string] $staging, [string] $zip) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    if (Test-Path $zip) { Remove-Item $zip -Force }
    [IO.Compression.ZipFile]::CreateFromDirectory($staging, $zip, [IO.Compression.CompressionLevel]::Optimal, $false)
}

function Get-Download([string] $path) {
    [ordered]@{
        url    = "$DownloadBase/$(Split-Path -Leaf $path)"
        sha256 = (Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant()
        size   = (Get-Item $path).Length
    }
}

function Get-PreviousTag([bool] $preRelease) {
    $ErrorActionPreference = 'Continue'
    $arguments = @('describe', '--tags', '--abbrev=0')
    if (-not $preRelease) { $arguments += @('--exclude', 'v*-*') }
    $previous = & git -C $root @arguments "$Tag^" 2>$null
    if ($LASTEXITCODE -ne 0) { return $null }
    $previous
}

function Write-Notes([string] $path, [bool] $preRelease) {
    $previous = Get-PreviousTag $preRelease
    if (-not $previous) {
        $lines = @('First release.')
    }
    else {
        $subjects = & git -C $root log --format='- %s' "$previous..$Tag"
        if ($LASTEXITCODE -ne 0) { throw "git log $previous..$Tag failed." }
        $lines = @("Changes since $previous`:", '') + @($subjects)
    }
    [IO.File]::WriteAllText($path, (($lines -join "`n") + "`n"))
}

if ($Tag -notmatch '^v(\d+\.\d+\.\d+)(-[0-9A-Za-z.-]+)?$') {
    throw "$Tag isn't a vX.Y.Z or vX.Y.Z-label tag."
}
$numeric = $Matches[1]
$version = $Tag.Substring(1)
$preRelease = [bool] $Matches[2]
$cargoVersion = Get-CargoVersion
if ($cargoVersion -ne $version) {
    throw "The tag says $version but Cargo.toml says $cargoVersion."
}

$out = Join-Path $root $Out
New-Item -ItemType Directory -Force $out | Out-Null
$built = Build-Release
$staging = Join-Path $out 'SterneGit'
New-Staging $built.Exe $staging

$setupName = "SterneGit-Setup-$version"
Invoke-Native 'Inno Setup' {
    & (Find-Iscc) /Q "/O$out" "/DAppVersion=$version" "/DNumericVersion=$numeric" "/DStaging=$staging" "/DSetupIcon=$($built.Icon)" (Join-Path $root 'installer\sternegit.iss')
}
$setup = Join-Path $out "$setupName.exe"
$portable = Join-Path $out "SterneGit-$version-portable.zip"
New-PortableZip $staging $portable

$latest = [ordered]@{
    version  = $version
    notes    = $NotesPage
    setup    = Get-Download $setup
    portable = Get-Download $portable
}
[IO.File]::WriteAllText((Join-Path $out 'latest.json'), (($latest | ConvertTo-Json) + "`n"))
Write-Notes (Join-Path $out 'notes.md') $preRelease

Write-Output "Packaged SterneGit $version into $out$(if ($preRelease) { ' as a pre-release' })."
