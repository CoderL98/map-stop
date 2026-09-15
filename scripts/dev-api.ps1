# Start map-stop Rust API (Windows PowerShell)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if (Test-Path "$Root\.env") {
    Get-Content "$Root\.env" | ForEach-Object {
        if ($_ -match '^\s*#' -or $_ -match '^\s*$') { return }
        $pair = $_.Split('=', 2)
        if ($pair.Length -eq 2) {
            $name = $pair[0].Trim()
            $val = $pair[1].Trim()
            [Environment]::SetEnvironmentVariable($name, $val, "Process")
        }
    }
}

# Prefer repo-local SQLite path on Windows if unset / still Linux path
$db = [Environment]::GetEnvironmentVariable("DATABASE_URL", "Process")
if (-not $db -or $db -like "*workspace*") {
    $dbPath = (Join-Path $Root "data\map-stop.db") -replace '\\', '/'
    [Environment]::SetEnvironmentVariable("DATABASE_URL", "sqlite:///$dbPath", "Process")
}

New-Item -ItemType Directory -Force -Path (Join-Path $Root "data") | Out-Null
Set-Location (Join-Path $Root "services\api")
Write-Host "Starting API (cargo run) ..."
cargo run
