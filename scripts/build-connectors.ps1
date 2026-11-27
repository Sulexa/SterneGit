$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$target = 'wasm32-unknown-unknown'
$sums = 'SHA256SUMS'

function Get-CrateName([string] $cargoToml) {
    $line = Select-String -Path $cargoToml -Pattern '^name\s*=\s*"([^"]+)"' | Select-Object -First 1
    $line.Matches[0].Groups[1].Value -replace '-', '_'
}

function Write-Sums([string] $dist) {
    $names = [string[]] @(Get-ChildItem $dist -File | Where-Object Name -ne $sums | ForEach-Object Name)
    [Array]::Sort($names, [StringComparer]::Ordinal)
    $lines = foreach ($name in $names) {
        $hash = (Get-FileHash (Join-Path $dist $name) -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $name"
    }
    [IO.File]::WriteAllText((Join-Path $dist $sums), (($lines -join "`n") + "`n"))
}

$families = @(
    @{ Folder = 'connectors'; Manifest = 'connector.json'; Noun = 'connector' },
    @{ Folder = 'readers'; Manifest = 'reader.json'; Noun = 'reader' }
)

$built = 0
foreach ($family in $families) {
    foreach ($crate in Get-ChildItem (Join-Path $root $family.Folder) -Directory) {
        $dist = Join-Path $crate.FullName 'dist'
        $manifest = Join-Path $dist $family.Manifest
        $cargoToml = Join-Path $crate.FullName 'Cargo.toml'
        if (-not ((Test-Path $manifest) -and (Test-Path $cargoToml))) { continue }

        $module = (Get-Content $manifest -Raw | ConvertFrom-Json).module
        Push-Location $crate.FullName
        try { & cargo build --release --target $target --color never }
        finally { Pop-Location }
        if ($LASTEXITCODE -ne 0) { throw "The $($crate.Name) $($family.Noun) failed to build." }

        $wasm = Join-Path $crate.FullName "target\$target\release\$(Get-CrateName $cargoToml).wasm"
        Copy-Item $wasm (Join-Path $dist $module) -Force
        Write-Sums $dist
        $size = (Get-Item (Join-Path $dist $module)).Length
        Write-Output "$($family.Folder)\$($crate.Name): dist\$module is $size bytes; $sums rewritten."
        $built++
    }
}

if ($built -eq 0) { throw 'No connector or reader with a dist manifest was found.' }
