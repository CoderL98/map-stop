#!/usr/bin/env bash
set -euo pipefail
API="${API:-http://127.0.0.1:8080/api}"

echo "== health =="
curl -sf "$API/health"
echo

echo "== meta/routing =="
curl -sf "$API/meta/routing" \
  | python3 -c 'import sys,json; d=json.load(sys.stdin); assert "provider" in d and "crs" in d and "engine" in d, d; print("ok provider=%s engine=%s crs=%s" % (d["provider"], d["engine"], d["crs"]))'
echo

echo "== login admin =="
TOKEN=$(curl -sf -X POST "$API/auth/login" -H 'Content-Type: application/json' \
  -d '{"login":"admin","password":"admin123"}' | python3 -c 'import sys,json; print(json.load(sys.stdin)["token"])')
AUTH="Authorization: Bearer $TOKEN"

echo "== geocode =="
curl -sf "$API/geocode?q=%E6%96%AD%E6%A1%A5&limit=2" \
  | python3 -c 'import sys,json; d=json.load(sys.stdin); assert isinstance(d,list); print("ok hits=%s" % len(d))'
echo

echo "== create system avoid =="
curl -sf -X POST "$API/system-points" -H "$AUTH" -H 'Content-Type: application/json' \
  -d '{"name":"烟测禁区","lat":30.26,"lon":120.15,"radius_m":200,"enabled":true}'
echo

echo "== plan with avoid (embedded or gaode) =="
# For opensource stub expect failure; otherwise success
PROVIDER=$(curl -sf "$API/meta/routing" | python3 -c 'import sys,json; print(json.load(sys.stdin)["provider"])')
set +e
PLAN_OUT=$(curl -s -w '\n%{http_code}' -X POST "$API/plan" -H "$AUTH" -H 'Content-Type: application/json' \
  -d '{"start":{"lat":30.22,"lon":120.10},"end":{"lat":30.30,"lon":120.20},"mode":"driving"}')
set -e
HTTP=$(echo "$PLAN_OUT" | tail -n1)
BODY=$(echo "$PLAN_OUT" | sed '$d')
if [[ "$PROVIDER" == "opensource" ]]; then
  echo "$BODY" | python3 -c 'import sys,json; d=json.load(sys.stdin); assert "未配置" in d.get("error",""), d; print("ok opensource stub rejected")'
else
  [[ "$HTTP" == "200" ]] || { echo "plan failed HTTP=$HTTP body=$BODY"; exit 1; }
  echo "$BODY" | python3 -c 'import sys,json; d=json.load(sys.stdin); p=d["properties"]; print("ok distance_m=%.0f duration_s=%.0f avoids=%s provider=%s pts=%s" % (p["distance_m"], p["duration_s"], p["avoid_count"], p.get("provider"), len(p["polyline"])))'
fi
echo
echo "SMOKE OK"
