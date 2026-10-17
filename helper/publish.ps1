# Publishes the helper the way it is distributed (self-contained, trimmed) and copies it next to
# the app, where the app looks for it. Run it from any directory:
#   helper\publish.ps1                      # into target\release
#   helper\publish.ps1 -Destination dist    # somewhere else
param(
    [string]$Destination = (Join-Path $PSScriptRoot '..\target\release'),
    [string]$Runtime = 'win-x64'
)
$ErrorActionPreference = 'Stop'

$out = Join-Path $PSScriptRoot 'Helper\bin\publish'
# mise pins the SDK version (mise.toml); a plain `dotnet` may have no SDK.
mise exec -- dotnet publish (Join-Path $PSScriptRoot 'Helper') -c Release -r $Runtime `
    --self-contained -p:PublishTrimmed=true -o $out
if ($LASTEXITCODE -ne 0) { throw 'dotnet publish failed' }

New-Item -ItemType Directory -Force $Destination | Out-Null
Copy-Item (Join-Path $out '*') $Destination -Recurse -Force
Write-Host "Helper copied to $Destination"
