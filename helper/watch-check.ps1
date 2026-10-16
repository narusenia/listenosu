# Starts the helper against the real lazer folder and prints every message it sends, so you can
# import / delete a beatmap in a running lazer and watch `changed` arrive. Read-only.
# Usage: powershell -File helper\watch-check.ps1 [seconds]   (default 180; Ctrl+C to stop early)
param([int]$Seconds = 180)
$ErrorActionPreference = 'Stop'

$dir = "$env:APPDATA\osu"
$exe = Join-Path $PSScriptRoot "Helper\bin\Release\net10.0\listenosu-helper.exe"
if (-not (Test-Path $exe)) { throw "Build first: mise exec -- dotnet build helper/Helper -c Release" }

$psi = [Diagnostics.ProcessStartInfo]::new($exe)
$psi.UseShellExecute = $false
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.StandardOutputEncoding = [Text.Encoding]::UTF8
$p = [Diagnostics.Process]::Start($psi)
$p.StandardInput.WriteLine('{"cmd":"open","data_dir":"' + ($dir -replace '\\', '\\') + '"}')

$end = [DateTime]::UtcNow.AddSeconds($Seconds)
$pending = $p.StandardOutput.ReadLineAsync()
while ([DateTime]::UtcNow -lt $end -and -not $p.HasExited) {
    if (-not $pending.Wait(500)) { continue }
    $j = $pending.Result | ConvertFrom-Json
    $pending = $p.StandardOutput.ReadLineAsync()
    $time = Get-Date -Format 'HH:mm:ss'
    if ($j.songs) { "$time $($j.type): songs=$($j.songs.Count) collections=$($j.collections.Count)" }
    else { "$time $($j.type): " + (($j.PSObject.Properties | Where-Object Name -ne 'type' | ForEach-Object { "$($_.Name)=$($_.Value)" }) -join ' ') }
}
$p.StandardInput.WriteLine('{"cmd":"quit"}')
$p.WaitForExit(5000) | Out-Null
"stopped. helper stderr:"
$p.StandardError.ReadToEnd()
