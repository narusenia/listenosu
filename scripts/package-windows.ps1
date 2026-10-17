# Builds the Windows release zip: listenosu.exe plus the helper next to it.
#   scripts\package-windows.ps1                 # dist\listenosu-<version>-win-x64.zip
param(
    [string]$Version = ((Select-String -Path (Join-Path $PSScriptRoot '..\Cargo.toml') -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value),
    [string]$Output = (Join-Path $PSScriptRoot '..\dist')
)
$ErrorActionPreference = 'Stop'
$root = Join-Path $PSScriptRoot '..'
$name = "listenosu-$Version-win-x64"
$stage = Join-Path $Output $name

cargo build --release -p listenosu --manifest-path (Join-Path $root 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

Remove-Item $Output -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
& (Join-Path $root 'helper\publish.ps1') -Destination $stage
Copy-Item (Join-Path $root 'target\release\listenosu.exe') $stage
Copy-Item (Join-Path $root 'LICENSE'), (Join-Path $root 'README.md'), (Join-Path $root 'README_ja.md') $stage

$zip = Join-Path $Output "$name.zip"
Compress-Archive -Path $stage -DestinationPath $zip
Write-Host "Created $zip"
