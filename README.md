# map-stop

网页版路线规划：系统躲避点库 + 用户自定义/上传审核，**硬避开**圆形禁区（绝对不穿行，不可达则中文报错）。

完整需求见 [`docs/REQUIREMENTS.md`](docs/REQUIREMENTS.md)。

## 技术栈

| 层 | 选型 |
|----|------|
| 前端 | SvelteKit + pnpm + TypeScript + Leaflet |
| 后端 | Rust (axum) + SQLite + JWT + argon2 |
| 底图 | OpenStreetMap 瓦片（保留 © OpenStreetMap 署名） |
| 地名搜索 | Nominatim（经 Rust `GET /api/geocode` 代理，服务端 User-Agent + 限流） |
| 算路 | **内嵌演示路网 + A\***（杭州西湖区域网格），服务端对折线二次几何校验 |

未使用高德 / 百度 / 腾讯 SDK。本环境未依赖 Docker；`docker-compose` 未纳入首版（可选后续加 GraphHopper/Valhalla）。

## 算路说明与限制（重要）

- **引擎**：`embedded-grid-astar`（Rust 内嵌），不是全国真实 OSM 路网。
- **演示区域**：杭州西湖附近约 `30.20–30.32°N, 120.08–120.22°E`。起终点请落在该范围（规划页会画出蓝色虚线框）。
- **网格密度**：约 80–100m 步进的矩形街道网格，用于本地硬避开演示；非真实道路几何。若需更密网格可后续调小 `dlat`/`dlon`。
- **硬避开**：规划时生效禁区 = 全部已启用系统点 ∪ 用户勾选的自定义点；边与圆相交则不可通行；无解返回「无法完全避开指定点位…」；结果折线再校验，禁止静默穿行。
- **出行方式**：driving / walking / cycling（影响时长估算速度，路网相同）。
- **上限**：单次生效躲避点 ≤ 50。
- 生产若需真实路网，可替换为自托管 GraphHopper / Valhalla（带 avoid 多边形）并由同一套校验逻辑兜底。

## 首次启动种子数据

- **管理员**：环境变量 `ADMIN_USERNAME` / `ADMIN_PASSWORD`（默认 `admin` / `admin123`）。
- **系统躲避点**：若 `system_points` 表为空，自动插入 3 个杭州演示框内启用点（半径约 90–120m）：
  - 断桥附近施工
  - 苏堤南口临时管制
  - 岳庙东侧围挡  
  可在管理端删除或停用。

## 快速启动

### 前置

- Rust（cargo）、Node 20+、pnpm
- Windows 可用 PowerShell 脚本；Linux/macOS/WSL 用 bash 脚本

### 1. 配置

```bash
cp .env.example .env
# 按需修改 ADMIN_USERNAME / ADMIN_PASSWORD / JWT_SECRET / DATABASE_URL
```

Windows PowerShell：

```powershell
Copy-Item .env.example .env
# 编辑 .env；DATABASE_URL 建议指向本机绝对路径，例如 sqlite:///C:/path/to/map-stop/data/map-stop.db
# 若仍为 /workspace/... ，scripts/dev-api.ps1 会自动改写为仓库下 data\map-stop.db
```

### 2. 启动 API

Linux / macOS / WSL：

```bash
mkdir -p data
./scripts/dev-api.sh
# 或：cd services/api && cargo run
# 默认 http://0.0.0.0:8080 ，路由前缀 /api
```

Windows：

```powershell
.\scripts\dev-api.ps1
# 或双击 / 运行 scripts\dev-api.bat
```

首次启动会创建 SQLite、种子管理员，并在无系统点时写入演示躲避点。

### 3. 启动前端

Linux / macOS / WSL：

```bash
cd apps/web
pnpm install
pnpm dev
# http://localhost:5173 （已代理 /api → :8080）
```

Windows：

```powershell
.\scripts\dev-web.ps1
# 或 scripts\dev-web.bat
```

### 冒烟测试

```bash
# API 已启动时
./scripts/smoke-test.sh
```

Windows：

```powershell
.\scripts\smoke-test.ps1
```

冒烟覆盖：health、登录、**geocode**、创建系统点、规划。

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
- `GET /api/geocode?q=` — Nominatim 代理（偏置演示 viewbox，`accept-language=zh`，约 1 req/s）

## 地图署名

地图数据 © [OpenStreetMap](https://www.openstreetmap.org/copyright) contributors（ODbL）。  
地名搜索使用 [Nominatim](https://nominatim.org/)（遵守使用政策：合理 User-Agent、限流）。

## 目录

```
apps/web          # SvelteKit
services/api      # Rust API
docs/             # REQUIREMENTS.md + CSV 模板
scripts/          # 本地启动与冒烟（.sh / .ps1 / .bat）
data/             # SQLite（本地，gitignore）
```
