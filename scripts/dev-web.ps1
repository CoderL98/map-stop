# Start map-stop SvelteKit frontend (Windows PowerShell)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location (Join-Path $Root "apps\web")

if (-not (Test-Path "node_modules")) {
    Write-Host "Installing deps (pnpm install) ..."
    pnpm install
}

Write-Host "Starting web on http://localhost:5173 ..."
pnpm dev --host 0.0.0.0 --port 5173
