# map-stop

地图躲避点位路线规划：系统躲避点库 + 用户自定义/上传审核，硬避开圆形禁区绕路规划。

## 文档

完整产品与技术说明见 [`docs/REQUIREMENTS.md`](docs/REQUIREMENTS.md)。

## 技术栈（规划）

- 后端：Rust
- 前端：SvelteKit + pnpm
- 地图 / 路由：OpenStreetMap + Leaflet + 开源路由引擎（硬避开）

## 状态

仓库已写入 v0.1 需求说明；工程骨架与 MVP 实现待继续推进。