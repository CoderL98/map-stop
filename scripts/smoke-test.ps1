# Smoke test against running API (Windows PowerShell)
$ErrorActionPreference = "Stop"
$Api = if ($env:API) { $env:API } else { "http://127.0.0.1:8080/api" }

Write-Host "== health =="
Invoke-RestMethod -Uri "$Api/health" | Out-Host

Write-Host "== meta/routing =="
$meta = Invoke-RestMethod -Uri "$Api/meta/routing"
Write-Host "ok provider=$($meta.provider) engine=$($meta.engine) crs=$($meta.crs)"

Write-Host "== login admin =="
$login = Invoke-RestMethod -Uri "$Api/auth/login" -Method POST -ContentType "application/json" -Body '{"login":"admin","password":"admin123"}'
$headers = @{ Authorization = "Bearer $($login.token)" }

Write-Host "== geocode =="
$geo = Invoke-RestMethod -Uri "$Api/geocode?q=%E6%96%AD%E6%A1%A5&limit=2"
Write-Host "ok hits=$($geo.Count)"

Write-Host "== create system avoid =="
Invoke-RestMethod -Uri "$Api/system-points" -Method POST -Headers $headers -ContentType "application/json" -Body '{"name":"烟测禁区","lat":30.26,"lon":120.15,"radius_m":200,"enabled":true}' | Out-Null

Write-Host "== plan with avoid =="
if ($meta.provider -eq "opensource") {
  try {
    Invoke-RestMethod -Uri "$Api/plan" -Method POST -Headers $headers -ContentType "application/json" -Body '{"start":{"lat":30.22,"lon":120.10},"end":{"lat":30.30,"lon":120.20},"mode":"driving"}'
    throw "expected opensource stub to fail"
  } catch {
    Write-Host "ok opensource stub rejected"
  }
} else {
  $plan = Invoke-RestMethod -Uri "$Api/plan" -Method POST -Headers $headers -ContentType "application/json" -Body '{"start":{"lat":30.22,"lon":120.10},"end":{"lat":30.30,"lon":120.20},"mode":"driving"}'
  $p = $plan.properties
  Write-Host ("ok distance_m={0:N0} duration_s={1:N0} avoids={2} provider={3}" -f $p.distance_m, $p.duration_s, $p.avoid_count, $p.provider)
}

Write-Host "SMOKE OK"
