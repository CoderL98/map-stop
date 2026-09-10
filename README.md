# map-stop

网页版路线规划：系统躲避点库 + 用户自定义/上传审核，**硬避开**圆形禁区（绝对不穿行，不可达则中文报错）。

完整需求见 [`docs/REQUIREMENTS.md`](docs/REQUIREMENTS.md)。

## 技术栈

| 层 | 选型 |
|----|------|
| 前端 | SvelteKit + pnpm + TypeScript + Leaflet |
| 后端 | Rust (axum) + SQLite + JWT + argon2 |
| 底图 | OpenStreetMap 瓦片（保留 © OpenStreetMap 署名） |
| 算路 | **内嵌演示路网 + A\***（杭州西湖区域网格），服务端对折线二次几何校验 |

未使用高德 / 百度 / 腾讯 SDK。本环境未依赖 Docker；`docker-compose` 未纳入首版（可选后续加 GraphHopper/Valhalla）。

## 算路说明与限制（重要）

- **引擎**：`embedded-grid-astar`（Rust 内嵌），不是全国真实 OSM 路网。
- **演示区域**：杭州西湖附近约 `30.20–30.32°N, 120.08–120.22°E`。起终点请落在该范围。
- **硬避开**：规划时生效禁区 = 全部已启用系统点 ∪ 用户勾选的自定义点；边与圆相交则不可通行；无解返回「无法完全避开指定点位…」；结果折线再校验，禁止静默穿行。
- **出行方式**：driving / walking / cycling（影响时长估算速度，路网相同）。
- **上限**：单次生效躲避点 ≤ 50。
- 生产若需真实路网，可替换为自托管 GraphHopper / Valhalla（带 avoid 多边形）并由同一套校验逻辑兜底。

## 快速启动

### 前置

- Rust（cargo）、Node 20+、pnpm

### 1. 配置

```bash
cp .env.example .env
# 按需修改 ADMIN_USERNAME / ADMIN_PASSWORD / JWT_SECRET / DATABASE_URL
```

### 2. 启动 API

```bash
mkdir -p data
./scripts/dev-api.sh
# 或：cd services/api && cargo run
# 默认 http://0.0.0.0:8080 ，路由前缀 /api
```

首次启动会创建 SQLite 并种子管理员（默认 `admin` / `admin123`）。

### 3. 启动前端

```bash
cd apps/web
pnpm install
pnpm dev
# http://localhost:5173 （已代理 /api → :8080）
```

### 冒烟测试

```bash
# API 已启动时
./scripts/smoke-test.sh
```

## CSV 模板

UTF-8：`名称,纬度,经度,半径米,备注`  
示例：[`docs/templates/avoid-points.csv`](docs/templates/avoid-points.csv)

## 主要 API

- `POST /api/auth/register` `POST /api/auth/login` `GET /api/auth/me`
- `GET|POST /api/system-points`（写操作需 admin）`POST /api/system-points/import`
- `GET|POST /api/custom-points` …
- `GET|POST /api/uploads` `POST /api/uploads/import`
- `GET /api/admin/uploads` `POST .../approve` `POST .../reject`
- `POST /api/plan` `GET /api/meta/demo-bounds`

## 地图署名

地图数据 © [OpenStreetMap](https://www.openstreetmap.org/copyright) contributors（ODbL）。

## 目录

```
apps/web          # SvelteKit
services/api      # Rust API
docs/             # REQUIREMENTS.md + CSV 模板
scripts/          # 本地启动与冒烟
data/             # SQLite（本地，gitignore）
```
