# Smoke test against running API (Windows PowerShell)
$ErrorActionPreference = "Stop"
$Api = if ($env:API) { $env:API } else { "http://127.0.0.1:8080/api" }

Write-Host "== health =="
$h = Invoke-RestMethod -Uri "$Api/health" -Method Get
Write-Host $h

Write-Host "== login admin =="
$login = Invoke-RestMethod -Uri "$Api/auth/login" -Method Post -ContentType "application/json" `
    -Body '{"login":"admin","password":"admin123"}'
$Token = $login.token
$Headers = @{ Authorization = "Bearer $Token" }

Write-Host "== geocode =="
$geo = Invoke-RestMethod -Uri "$Api/geocode?q=%E6%96%AD%E6%A1%A5&limit=2" -Method Get
if (-not $geo -or $geo.Count -lt 1) { throw "geocode returned no results" }
Write-Host ("ok hits={0} first={1}" -f $geo.Count, $geo[0].display_name)

Write-Host "== create system avoid =="
Invoke-RestMethod -Uri "$Api/system-points" -Method Post -Headers $Headers -ContentType "application/json" `
    -Body '{"name":"烟测禁区","lat":30.26,"lon":120.15,"radius_m":200,"enabled":true}' | Out-Null
Write-Host "ok"

Write-Host "== plan with avoid =="
$plan = Invoke-RestMethod -Uri "$Api/plan" -Method Post -Headers $Headers -ContentType "application/json" `
    -Body '{"start":{"lat":30.22,"lon":120.10},"end":{"lat":30.30,"lon":120.20},"mode":"driving"}'
$p = $plan.properties
Write-Host ("ok distance_m={0:N0} duration_s={1:N0} avoids={2} pts={3}" -f $p.distance_m, $p.duration_s, $p.avoid_count, $p.polyline.Count)

Write-Host "SMOKE OK"
