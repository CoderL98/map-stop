#!/usr/bin/env bash
set -euo pipefail
API="${API:-http://127.0.0.1:8080/api}"

echo "== health =="
curl -sf "$API/health"
echo

echo "== login admin =="
TOKEN=$(curl -sf -X POST "$API/auth/login" -H 'Content-Type: application/json' \
  -d '{"login":"admin","password":"admin123"}' | python3 -c 'import sys,json; print(json.load(sys.stdin)["token"])')
AUTH="Authorization: Bearer $TOKEN"

echo "== create system avoid =="
curl -sf -X POST "$API/system-points" -H "$AUTH" -H 'Content-Type: application/json' \
  -d '{"name":"烟测禁区","lat":30.26,"lon":120.15,"radius_m":200,"enabled":true}'
echo

echo "== plan with avoid =="
curl -sf -X POST "$API/plan" -H "$AUTH" -H 'Content-Type: application/json' \
  -d '{"start":{"lat":30.22,"lon":120.10},"end":{"lat":30.30,"lon":120.20},"mode":"driving"}' \
  | python3 -c 'import sys,json; d=json.load(sys.stdin); p=d["properties"]; print("ok distance_m=%.0f duration_s=%.0f avoids=%s pts=%s" % (p["distance_m"], p["duration_s"], p["avoid_count"], len(p["polyline"])))'
echo
echo "SMOKE OK"
