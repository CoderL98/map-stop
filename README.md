# map-stop

网页版路线规划：系统躲避点库 + 用户自定义/上传审核，**硬避开**圆形禁区（绝对不穿行，不可达则中文报错）。

完整需求见 [`docs/REQUIREMENTS.md`](docs/REQUIREMENTS.md)。

## 技术栈

| 层 | 选型 |
|----|------|
| 前端 | SvelteKit + pnpm + TypeScript；规划 / 管理 / 上传页优先 **高德 JS API 2.0**（有 `AMAP_JS_KEY` 时），否则 Leaflet + OSM |
| 后端 | Rust (axum) + SQLite + JWT + argon2 |
| 算路 | 可插拔 `RoutingProvider`：`gaode`（默认有 Key）/ `embedded`（杭州网格 A*）/ `opensource`（二期 stub） |
| 地名搜索 | 高德输入提示/地理编码（gaode）或 Nominatim 代理（embedded） |

## 算路 Provider（重要）

环境变量 `ROUTING_PROVIDER=gaode|embedded|opensource`：

| 值 | 行为 |
|----|------|
| `gaode` | 高德 Web 服务路径规划 v5（驾车/步行/骑行）。需 `AMAP_WEB_KEY`；无 Key 时**回退 embedded** 并在 `/api/meta/routing` 警告 |
| `embedded` | 内嵌杭州西湖演示网格 A*，始终可用，无需 Key |
| `opensource` | 预留 GraphHopper/Valhalla；当前返回明确「未配置」错误 |

未设置 `ROUTING_PROVIDER` 时：有 `AMAP_WEB_KEY` → `gaode`，否则 `embedded`。

**硬避开（所有 provider）**：服务端对结果折线做圆形禁区二次校验；相交则中文失败，禁止静默穿行。

**高德避让**：

- 驾车：原生 `avoidpolygons`（圆近似多边形）+ Rust 校验
- 步行/骑行：官方 API **无** avoidpolygons → 仍请求算路，但 Rust 校验失败则报错（含说明）

**坐标系**：高德路径下点位按 **GCJ-02** 存储与展示；勿混用未转换的 WGS84/OSM 点击坐标。详见 `/api/meta/routing` 的 `crs` 字段。

### 申请高德 Key

1. 注册 [高德开放平台](https://console.amap.com/dev/key/app)
2. 创建应用，添加两类 Key：
   - **Web服务** → `AMAP_WEB_KEY`（路径规划、地理编码、输入提示）
   - **Web端(JS API)** → `AMAP_JS_KEY`（前端底图）；推荐配置安全密钥 `AMAP_SECURITY_JS_CODE`
3. 个人开发者有免费配额；**商用请遵守高德服务条款并购买相应授权**，勿将 Key 提交进仓库

切换到二期自托管开源路由：实现同一 `RoutingProvider` 接口后设 `ROUTING_PROVIDER=opensource`（当前为 stub）。

## 快速启动

### 前置

- Rust（cargo）、Node 20+、pnpm

### 1. 配置

```bash
cp .env.example .env
# 至少设置 JWT_SECRET / ADMIN_* / DATABASE_URL
# 使用高德：填写 AMAP_WEB_KEY（及 AMAP_JS_KEY），ROUTING_PROVIDER=gaode
```

### 2. 启动 API

```bash
mkdir -p data
./scripts/dev-api.sh
# 默认 http://0.0.0.0:8080 ，路由前缀 /api
```

### 3. 启动前端

```bash
cd apps/web && pnpm install && pnpm dev
# http://localhost:5173 （代理 /api → :8080）
```

### 冒烟测试

```bash
./scripts/smoke-test.sh
```

无 Key 时冒烟走 `embedded`；有 Key 时可设 `ROUTING_PROVIDER=gaode` 验证高德算路。

## 环境变量

| 变量 | 说明 |
|------|------|
| `DATABASE_URL` | SQLite URL |
| `JWT_SECRET` | JWT 密钥 |
| `LISTEN_ADDR` | 默认 `0.0.0.0:8080` |
| `CORS_ORIGIN` | 默认 `http://localhost:5173` |
| `ADMIN_USERNAME` / `ADMIN_PASSWORD` | 种子管理员 |
| `ROUTING_PROVIDER` | `gaode` \| `embedded` \| `opensource` |
| `AMAP_WEB_KEY` | 高德 Web 服务 Key |
| `AMAP_JS_KEY` | 高德 JS API Key（前端） |
| `AMAP_SECURITY_JS_CODE` | JS 安全密钥（可选） |

## 主要 API

- 认证 / 系统点 / 自定义点 / 上传审核：同前
- `POST /api/plan` — 规划（properties 含 `provider`、`crs`）
- `GET /api/meta/routing` — 当前引擎、CRS、限制、JS Key（供前端）
- `GET /api/meta/demo-bounds` — 同上（兼容旧前端）
- `GET /api/geocode?q=` — 高德或 Nominatim

## 地图与版权

- **高德**：须遵守 [高德开放平台条款](https://lbs.amap.com/)；Key 与配额由调用方自行申请与付费；商用需相应授权。
- **OSM / Leaflet 回退**：© [OpenStreetMap](https://www.openstreetmap.org/copyright)（ODbL）；Nominatim 遵守使用政策。
- **开源算路（二期）**：GraphHopper / Valhalla + OSM，接口已预留。

## 目录

```
apps/web          # SvelteKit
services/api      # Rust API（routing/{mod,embedded,gaode,opensource}.rs）
docs/             # REQUIREMENTS.md + CSV 模板
scripts/          # 本地启动与冒烟
data/             # SQLite（本地，gitignore）
```
